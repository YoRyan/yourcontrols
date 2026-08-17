var isHttp = true;

async function httpMain() {
    if (!httpCheckExclusive()) {
        document.documentElement.innerHTML = `<p>Only one browser can access the web server at a time.</p>
<p>If you've already closed the original tab that connected, please stop and restart the program.</p>
<p><button onclick="httpExitProcess(this);">Request Shutdown</button></p>`;
        return;
    }

    const wsMessage = httpWebSocket("/ws/message", "message");
    // It's important to have this shim in place before the rest of the JavaScript executes, so it must be placed above any await.
    external = {};
    external.invoke = async function (json) {
        // We may get called before the websocket has finished opening.
        (await wsMessage).send(json);
    };

    const wsInvoke = await httpWebSocket("/ws/invoke", "invoke");
    wsInvoke.addEventListener("message", event => {
        eval(event.data);
    });
}

function httpCheckExclusive() {
    const clientId = Math.floor(Math.random() * 0xffffffff) + "";
    const request = new XMLHttpRequest();
    request.open("GET", "/is-exclusive", false);
    request.setRequestHeader("X-Client-ID", clientId);
    request.send(null);
    switch (request.status) {
        case 200:
            return true;
        case 409:
        default:
            return false;
    }
}

async function httpExitProcess(el) {
    const response = await fetch("/process", { method: "DELETE" });
    if (response.status === 200) {
        el.disabled = true;
    }
}

async function httpWebSocket(url, ...protocols) {
    const socket = new WebSocket(url, ...protocols);
    await new Promise((resolve, reject) => {
        socket.addEventListener("open", resolve);
        socket.addEventListener("error", reject);
    });
    return socket;
}

httpMain();
