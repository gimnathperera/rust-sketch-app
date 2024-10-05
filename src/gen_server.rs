use std::collections::HashMap;
use std::net::SocketAddr;
use crate::line::Line;
use tokio::sync::mpsc::{self, Sender, Receiver};

#[derive(Debug, Clone)]
pub enum GSMsg {
    NewClient((SocketAddr, Sender<GSMsg>)),
    DeleteClient(SocketAddr),
    NewLine(Line),
    Clear
}

pub fn spawn() -> Sender<GSMsg> {
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(gen_server(rx));
    tx
}

async fn gen_server(mut rx: Receiver<GSMsg>) {
    let mut clients: HashMap<SocketAddr, Sender<GSMsg>> = HashMap::new();
    let mut lines: Vec<Line> = vec![];
    
    while let Some(msg) = rx.recv().await {
	match msg {
	    GSMsg::NewClient((addr, c_tx)) => {
		for line in &lines {
		    let ret = c_tx.send(GSMsg::NewLine(line.clone())).await;
		    if let Err(err) = ret {
			tracing::warn!("Client {addr} send error: {err}");
		    }
		}
		clients.insert(addr, c_tx);		
		tracing::info!("NewClient {addr}");
	    },
	    GSMsg::NewLine(line) => {
		send_all(&mut clients, &GSMsg::NewLine(line.clone())).await;
		lines.push(line);
	    },
	    GSMsg::DeleteClient(addr) => {
		clients.remove(&addr);		
		tracing::info!("Client {addr} removed");
	    },
	    GSMsg::Clear => {
		send_all(&mut clients, &GSMsg::Clear).await;
		lines.clear();
	    }
	}
    }
}

async fn send_all(
    clients: &mut HashMap<SocketAddr, Sender<GSMsg>>,
    msg: &GSMsg
) {
    let mut to_remove : Vec<SocketAddr> = vec![];
    
    for (addr, ref mut tx) in clients.iter() {
	let ret = tx.send(msg.clone()).await;
	if let Err(err) = ret {	    
	    tracing::warn!("Client {addr} abruptly disconnected: {err}");
	    to_remove.push(*addr);
	}
    }
    
    for addr in to_remove {
	clients.remove(&addr);
    }
}
