external = {};
external.invoke = function (body) {
    fetch("/invoke", {
        method: "PUT",
        headers: {
            "Content-Type": "application/json",
        },
        body,
    });
}

async function httpMessageReceive() {
    const response = await fetch("/invoke");
    let timeout;
    switch (response.status) {
        case 200:
            eval(await response.text());
            timeout = 0;
            break;
        case 204:
        default:
            timeout = 10;
            break;
    }
    setTimeout(httpMessageReceive, timeout);
}
httpMessageReceive();
