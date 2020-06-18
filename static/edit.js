(async () => {
    require.config({
        paths: {
            vs:
                "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.19.2/min/vs",
        },
    });

    window.MonacoEnvironment = {
        getWorkerUrl: function (workerId, label) {
            return `data:text/javascript;charset=utf-8,${encodeURIComponent(`
          self.MonacoEnvironment = {
            baseUrl: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.19.2/min/'
          };
          importScripts('https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.19.2/min/vs/base/worker/workerMain.js');`)}`;
        },
    };

    const title = stripPrefix(window.location.pathname, "/edit/");
    let response = await fetch("/api/article_info/" + title, {
        method: "GET",
    });

    // TODO: show error
    if (response.status !== 200) return;

    const article_info = await response.json();
    initMonaco(article_info.markdown);

    document
        .querySelector("#save-button")
        .addEventListener("click", async () => {
            const response = await fetch("/api/edit/" + title, {
                method: "PUT",
                credentials: "same-origin",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    markdown: window.editor.getValue(),
                    oid: article_info.oid,
                }),
            });

            // TODO: handle diff if somebody else commited before
            console.log(response);
            if (response.status === 200) {
                const body = await response.json();
                console.log(body);
                switch (body.type) {
                    case "ok":
                        window.location = "/wiki/" + title;
                        break;
                    case "diff":
                        console.error("Got diff");
                        break;
                }
            } else {
                console.error(response);
            }
        });
})();

function stripPrefix(s, prefix) {
    return s.indexOf(prefix) === 0 ? s.slice(prefix.length) : s;
}

function initMonaco(text) {
    require(["vs/editor/editor.main"], function () {
        window.editor = monaco.editor.create(
            document.querySelector("#editor"),
            {
                value: text,
                language: "markdown",
                minimap: {
                    enabled: false,
                },
            }
        );
    });
}

(() => {
    const fileInput = document.querySelector(".file-input");
    fileInput.addEventListener("change", async () => {
        const file = fileInput.files[0];
        const data = new FormData();
        data.append("file", file);
        const resp = await fetch("/storage", {
            method: "PUT",
            credentials: "same-origin",
            body: data,
        });
        if (resp.status !== 200) {
            console.error(resp);
            return;
        }

        const body = await resp.json();
        document.querySelector(
            ".file-link"
        ).innerHTML = `<a href=${body.url}>${body.url}</a>`;
    });
})();

function switchTo(elt) {
    const classList = elt.classList;
    const wasActive = classList.contains("hidden");
    if (!wasActive) {
        classList.add("active");
        classList.remove("hidden");
    }

    return wasActive;
}

const tabs = new Map([
    [
        document.querySelector("#edit-button"),
        document.querySelector("#editor-tab"),
    ],
    [
        document.querySelector("#preview-button"),
        document.querySelector("#preview-tab"),
    ],
]);

function switchTo(targetButton) {
    const targetTab = tabs.get(targetButton);
    if (!targetTab.classList.contains("hidden")) {
        return false;
    }
    targetButton.classList.add("active");

    targetTab.classList.remove("hidden");

    for (const [button, tab] of tabs) {
        if (tab !== targetTab) {
            button.classList.remove("active");
            tab.classList.add("hidden");
        }
    }

    return true;
}

document.querySelector("#edit-button").addEventListener("click", (evt) => {
    switchTo(evt.target);
});

document
    .querySelector("#preview-button")
    .addEventListener("click", async (evt) => {
        const needsRender = switchTo(evt.target);
        if (needsRender) {
            const article = document.querySelector("#preview-tab > article");
            article.innerHTML = "Rendering preview";
            const response = await fetch("/api/preview", {
                method: "PUT",
                credentials: "same-origin",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    markdown: window.editor.getValue(),
                }),
            });

            if (response.status === 200) {
                const json = await response.json();
                article.innerHTML = json.rendered;
            }
        }
    });
