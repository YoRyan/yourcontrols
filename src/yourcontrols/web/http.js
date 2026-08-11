var isHttp = true,
    httpClientId = Math.floor(Math.random() * 0xffffffff) + "";

external = {};
external.invoke = function (json) {
    const blob = new Blob([json], { type: "application/json" });
    const request = new XMLHttpRequest();
    request.open("PUT", "/invoke", false);
    request.setRequestHeader("X-Client-ID", httpClientId);
    request.send(blob);
}

async function httpExitProcess(el) {
    const response = await fetch("/process", { method: "DELETE" });
    if (response.status === 200) {
        el.disabled = true;
    }
}

function httpMessageReceive() {
    const lockedOutHtml = `<p>Only one browser can access the web server at a time.</p>
<p>If you've already closed the original tab that connected, please stop and restart the program.</p>
<p><button onclick="httpExitProcess(this);">Request Shutdown</button></p>`;

    loop: do {
        const request = new XMLHttpRequest();
        request.open("GET", "/invoke", false);
        request.setRequestHeader("X-Client-ID", httpClientId);
        request.send(null);

        switch (request.status) {
            case 200:
                eval(request.responseText);
                break;
            case 409:
                document.documentElement.innerHTML = lockedOutHtml;
                break loop;
            case 204:
            default:
                setTimeout(httpMessageReceive, 50);
                break loop;
        }
    } while (true);
}
httpMessageReceive();
