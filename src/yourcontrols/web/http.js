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

function httpMessageReceive() {
    const badClientId = `Only one browser can use the web server at a time.
If you've already closed the original tab that connected, please restart the program.`;

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
                document.documentElement.innerHTML =
                    `<pre>${badClientId}</pre>`;
                break loop;
            case 204:
            default:
                setTimeout(httpMessageReceive, 50);
                break loop;
        }
    } while (true);
}
httpMessageReceive();
