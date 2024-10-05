const colors = [
    "#000000",
    "#1D2B53",
    "#7E2553",
    "#008751",
    "#AB5236",
    "#5F574F",
    "#C2C3C7",
    "#FFF1E8",
    "#FF004D",
    "#FFA300",
    "#FFEC27",
    "#00E436",
    "#29ADFF",
    "#83769C",
    "#FF77A8",
    "#FFCCAA"
];

const initUI = (colorsDiv, selectedColor) => {
    for (const color of colors) {
	(color => {
	    const div = document.createElement("div");
	    div.setAttribute("class", "color");	
	    div.setAttribute("style", "background: "+color+";");
	    div.onclick = evt => { selectedColor.value = color; };
	    colorsDiv.appendChild(div);
	})(color);
    }
};

//Get Mouse Position
const getMousePoint = (canvas, evt, color) => {
    var rect = canvas.getBoundingClientRect();
    const x = Math.floor(canvas.width * (evt.clientX - rect.left) / rect.width);
    const y = Math.floor(canvas.height * (evt.clientY - rect.top) / rect.width);
    return { x, y, color };
};

const updateLine = (ctx, line) => {
    ctx.strokeStyle = line[0].color;
    ctx.beginPath();
    ctx.moveTo(line[0].x, line[0].y);
    for(let i=1; i < line.length; ++i) {
	ctx.lineTo(line[i].x, line[i].y);
    }
    ctx.stroke();
};

const updateCanvas = (canvas, ctx, lines) => {
    ctx.fillStyle = "black";
    ctx.fillRect(0, 0, canvas.width, canvas.height);    
    for(let line of lines) {
	updateLine(ctx, line);
    }
};

const initDrawing = (canvas, clearButton, errButton, selectedColor, ws) => {
    const ctx = canvas.getContext("2d");
    ctx.lineWidth = 2;
    let lines = [];
    let currentLine = null;

    const resetCanvas = () => {
	ctx.fillStyle = "black";
	ctx.fillRect(0, 0, canvas.width, canvas.height);
    };
    resetCanvas();
    
    clearButton.onclick = evt => {
	ws.send(JSON.stringify({ t: "clear" }));
    };

    if (errButton) {
	errButton.onclick = evt => {
	    ws.send(JSON.stringify({ t: "error", msg: "this is an error" }));
	};
    }
    
    canvas.addEventListener("mousedown", evt => {
	const pt = getMousePoint(canvas, evt, selectedColor.value);
	currentLine = [ pt ];
	ws.send(JSON.stringify({ ...pt, t: "moveTo" }));
    });

    canvas.addEventListener("mousemove", evt => {
	if(evt.buttons === 1) {
	    const pt = getMousePoint(canvas, evt, selectedColor.value);
	    currentLine.push(pt);
	    updateLine(ctx, currentLine);
	    ws.send(JSON.stringify({ ...pt, t: "lineTo" }));
	}
    });

    canvas.addEventListener("mouseup", evt => {
	// console.log(currentLine);
	currentLine = null;
	ws.send(JSON.stringify({ t: "stroke"})); 
    });

    ws.addEventListener('message', function (event) {
	let j = JSON.parse(event.data);
	console.log(j);
	if (j.t === "line") {
	    let line = j.line.map(a => ({ x: a[0], y: a[1], color: a[2] }));
	    lines.push(line);
	    updateCanvas(canvas, ctx, lines);
	} else if (j.t == "clear") {
	    lines = [];
	    resetCanvas();	    
	}
    });
};

const initWs = errorBox => {
    const socket = new WebSocket('ws://'+ location.host + '/ws');

    socket.addEventListener('open', function (event) {
	console.log("Open", event);
    });
    socket.addEventListener('error', function (event) {
	console.log("Error:", event);
	errorBox.className = "visible";
	errorBox.innerHTML = "Error: " + event.message;
    });
    socket.addEventListener("close", (event) => {
	console.log("Close:", event);
	errorBox.className = "visible";
	errorBox.innerHTML = "Disconnected: server closed connexion";
    });
    return socket;
};

window.onload = () => {
    const colorsDiv = document.querySelector("#colors");
    const selectedColor = document.querySelector("#selectedColor");
    const clearButton = document.querySelector("#clearButton");
    const errButton = document.querySelector("#errButton");
    const errorBox = document.querySelector("#errorBox");
    const canvas = document.querySelector("#canvas");
    initUI(colorsDiv, selectedColor);
    let ws = initWs(errorBox);
    initDrawing(canvas, clearButton, errButton, selectedColor, ws);
};
