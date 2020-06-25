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
    const response = await fetch("/api/article_info/" + title, {
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
            },
        );
    });
}

function e(ty, attrs, children) {
    const ret = document.createElement(ty);

    if (attrs !== undefined) Object.assign(ret, attrs);

    if (children !== undefined) ret.append(...children);

    return ret;
}

function insertTextAtCursor(editor, text) {
    const selection = editor.getSelection();
    const range = new monaco.Range(
        selection.startLineNumber,
        selection.startColumn,
        selection.endLineNumber,
        selection.endColumn,
    );
    const id = { major: 1, minor: 1 };
    const op = {
        identifier: id,
        range: range,
        text: text,
        forceMoveMarkers: true,
    };
    editor.executeEdits("my-source", [op]);
}

function clearFileList(elt) {
    elt.value = "";
}

function addFileInput() {
    const uploadFile = async (evt) => {
        const fileInput = evt.target;
        const listElt = fileInput.closest("li");

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
            clearFileList(fileInput);
            return;
        }

        const body = await resp.json();

        listElt.append(
            e("a", { href: body.url, textContent: body.url }),
            e("button", {
                onclick: () =>
                    insertTextAtCursor(
                        window.editor,
                        `![Enter alternate description here](${body.url})`,
                    ),
                textContent: "Insert markdown",
            }),
            e("button", {
                onclick: () => listElt.remove(),
                textContent: "Delete",
            }),
        );
        addFileInput();
    };

    const listElt = e("li", {}, [
        e("input", {
            type: "file",
            classList: "file-input",
            onchange: uploadFile,
        }),
    ]);

    document.querySelector("#file-list").append(listElt);
}

addFileInput();

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
