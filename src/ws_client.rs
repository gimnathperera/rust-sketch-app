use axum::extract::ws::{Message, WebSocket};
use std::net::SocketAddr;
use tokio::sync::mpsc::{self, Sender};
use serde::{Serialize, Deserialize};
use core::ops::ControlFlow;
use anyhow::{Result,anyhow};

use crate::gen_server::GSMsg;
use crate::line::{Line, simplify_line};
		  
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t")]
enum JMsg {
    #[serde(rename = "clear")]     
    Clear,
    #[serde(rename = "moveTo")] 
    MoveTo { x: f32, y: f32, color: String },
    #[serde(rename = "lineTo")]     
    LineTo { x: f32, y: f32, color: String },
    #[serde(rename = "stroke")]     
    Stroke,
    #[serde(rename = "line")]     
    Line { line: Vec<(f32,f32,String)> }
}

pub async fn handle_socket(
    socket: WebSocket,
    who: SocketAddr,
    gs_tx: Sender<GSMsg>
) {
    match handle_socket_(socket, who, gs_tx).await {
	Ok(()) => {},
	Err(err) => tracing::warn!("{who}: WS Handler error: {err}")
    }
}

pub async fn handle_socket_(
    mut socket: WebSocket,
    who: SocketAddr,
    gs_tx: Sender<GSMsg>
) -> Result<()> {
    let (chan_tx, mut chan_rx) = mpsc::channel(32); 
    gs_tx.send(GSMsg::NewClient((who, chan_tx))).await?;
    let mut line : Line = vec![];

    loop {
	tokio::select! {
	    Some(msg) = socket.recv() => {
		let Ok(msg) = msg else {
		    tracing::warn!("{who}: Error receiving packet: {msg:?}");
		    continue;
		};
		match process_ws_msg(&gs_tx, &who, &mut line, msg).await? {
		    ControlFlow::Break(()) => break Ok(()),
		    ControlFlow::Continue(()) => {}
		}
	    },
	    Some(msg) = chan_rx.recv() => {
		process_gs_msg(&mut socket, &who, msg).await?
	    },
	    else => break Err(anyhow!("{who}: Connection lost unexpectedly."))
	}
    }
}

async fn process_gs_msg(
    socket: &mut WebSocket,
    who: &SocketAddr,
    msg: GSMsg
) -> Result<()> {
    match msg {
	GSMsg::NewLine(line) => {
	    socket.send(Message::Text(line_to_json(&line)?)).await?;
	},
	GSMsg::Clear => {
	    let msg = serde_json::to_string(&JMsg::Clear)?;
	    socket.send(Message::Text(msg)).await?;
	},
	msg => {
	    tracing::info!("{who} should not get this: {:?}", msg);
	}
    }
    Ok(())
}

async fn process_ws_msg(
    gs_tx: &Sender<GSMsg>,
    who: &SocketAddr,
    line: &mut Line,
    msg: Message
) -> Result<ControlFlow<(),()>> {
    match msg {
	Message::Text(text) => {
	    tracing::debug!("{who}: sent: {text}");
	    match serde_json::from_str(&text) {
		Ok(json) => {
		    tracing::debug!("{who}: got json: {json:?}");
		    match handle_ws_msg(line, json) {
			Ok(Some(req)) => gs_tx.send(req).await?,
			Ok(None) => {},
			Err(err) => {
			    tracing::warn!("{who}: message error: {err}");
			}
		    }
		},
		Err(err) => tracing::warn!("{who}: can't parse JSON: {err} in: {text}")

	    }
	},
	Message::Close(close) => {
	    tracing::info!("{who}: closing: {close:?}");
	    gs_tx.send(GSMsg::DeleteClient(*who)).await?;
	    return Ok(ControlFlow::Break(()));
	},
	_ => {
	    tracing::warn!("{who}: can't handle message: {msg:?}");
	}
    };
    Ok(ControlFlow::Continue(()))
}

fn handle_ws_msg(
    line: &mut Line,
    msg: JMsg
) -> Result<Option<GSMsg>> {
    match msg {
	JMsg::Clear => Ok(Some(GSMsg::Clear)),
	JMsg::MoveTo { x, y, color } => {
	    line.clear();
	    line.push((x, y, parse_color(color)?));
	    Ok(None)
	},
	JMsg::LineTo { x, y, color } => {
	    line.push( (x, y, parse_color(color)?) );
	    Ok(None)
	},
	JMsg::Stroke if line.len() > 1 => {
	    let simple_line = simplify_line(line);
	    line.clear();
	    Ok(Some(GSMsg::NewLine(simple_line)))
	},
	JMsg::Stroke => Err(anyhow!("can't stroke a 1 point 'line'")),
	JMsg::Line{..} => Err(anyhow!("recieved a line message O_o"))
    }
}

fn line_to_json(line: &Line) -> Result<String> {
    let line = line.iter()
	.map(| (x, y, c) | (*x, *y, format!("#{:06x}", c)))
	.collect();    
    let json = serde_json::to_string(&JMsg::Line{ line })?;
    Ok(json)
}

fn parse_color(s: String) -> Result<u32> {
    if s.len() != 7 || &s[0..1] != "#" {
	Err(anyhow!("badly formated color"))
    } else {
	u32::from_str_radix(&s[1..], 16)
	    .map_err(|_| anyhow!("unable to parse color"))
    }
}
