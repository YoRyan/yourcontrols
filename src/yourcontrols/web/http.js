isHttp = true;

external = {};
external.invoke = function (json) {
    const blob = new Blob([json], { type: "application/json" });
    const request = new XMLHttpRequest();
    request.open("PUT", "/invoke", false);
    request.send(blob);
}

function httpMessageReceive() {
    loop: do {
        const request = new XMLHttpRequest();
        request.open("GET", "/invoke", false);
        request.send(null);

        switch (request.status) {
            case 200:
                eval(request.responseText);
                break;
            case 204:
            default:
                break loop;
        }
    } while (true);
    setTimeout(httpMessageReceive, 50);
}
httpMessageReceive();
