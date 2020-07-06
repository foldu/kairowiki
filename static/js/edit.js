window.addEventListener("load", async () => {
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

    notify("test", "Test");

    // TODO: show error
    if (response.status !== 200) {
        notify("Error", "Could not fetch article information");
        return;
    }

    const articleInfo = await response.json();
    const editor = await initMonaco(articleInfo.markdown);
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

    window.model = {
        editor,
        tabs,
        activeEditor: editor,
        diffEditor: null,
        articleInfo,
        title,
        getValue: () => editor.getValue(),
    };
});

document.querySelector("#save-button").addEventListener("click", async () => {
    const body = {
        markdown: window.model.getValue(),
        oid: window.model.articleInfo.oid,
        rev: window.model.articleInfo.rev,
        commitMsg: document.querySelector("#commit-msg").value,
    };
    const response = await fetch("/api/edit/" + window.model.title, {
        method: "PUT",
        credentials: "same-origin",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify(body),
    });

    if (response.status === 200) {
        const body = await response.json();
        switch (body.type) {
            case "noConflict":
                window.location = "/wiki/" + window.model.title;
                break;
            case "merged":
                window.model.articleInfo = {
                    ...window.model.articleInfo,
                    oid: body.oid,
                    rev: body.rev,
                };
                notify("Merge Conflict", "Your changes were auto-merged");
                switchToDiff(body);
                break;
            default:
                console.error("Unhandled case", body.type);
                break;
        }
    } else {
        notify("Error", "Could not save changes, please try again later");
        console.error(response);
    }
});

function switchToDiff(body) {
    if (window.model.diffEditor === null) {
        const diffDiv = document.querySelector("#diff-editor");
        document.querySelector("#editor").classList.add("hidden");
        diffDiv.classList.remove("hidden");
        window.model.diffEditor = monaco.editor.createDiffEditor(diffDiv);
    }

    const modified = monaco.editor.createModel(body.merged);
    window.model.getValue = () => modified.getValue();
    window.model.activeEditor = window.model.diffEditor.getModifiedEditor();

    window.model.diffEditor.setModel({
        original: monaco.editor.createModel(window.model.editor.getValue()),
        modified: modified,
    });
}

function stripPrefix(s, prefix) {
    return s.indexOf(prefix) === 0 ? s.slice(prefix.length) : s;
}

async function initMonaco(text) {
    return new Promise((resolve) => {
        require(["vs/editor/editor.main"], function () {
            const editor = monaco.editor.create(
                document.querySelector("#editor"),
                {
                    value: text,
                    language: "markdown",
                    minimap: {
                        enabled: false,
                    },
                },
            );
            resolve(editor);
        });
    });
}

function notify(title, body) {
    const elt = $e("li", {}, [$e("div", {}, title), $e("div", {}, body)]);
    const removeNotification = () => {
        elt.remove();
    };
    elt.onclick = removeNotification;
    setTimeout(removeNotification, 5 * 1000);

    document.querySelector("#notifications").append(elt);
}

function $e(ty, attrs, children) {
    const ret = document.createElement(ty);

    if (attrs !== undefined) {
        Object.assign(ret, attrs);
    }

    if (children !== undefined) {
        if (typeof children === "string") {
            ret.textContent = children;
        } else {
            ret.append(...children);
        }
    }

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
    editor.executeEdits("kairowiki", [op]);
}

function insertImageLink(editor, url) {
    const md = `![](${body.url})`;
    const selection = editor.getSelection();
    const range = new monaco.Range(
        selection.startLineNumber,
        selection.startColumn,
        selection.endLineNumber,
        selection.endColumn,
    );
    const identifier = { major: 1, minor: 1 };
    const insertOp = {
        identifier,
        range,
        text: md,
        forceMoveMarkers: true,
    };

    // TODO: figure out how to move the cursor to ![here](...)
    editor.executeEdits("kairowiki", [insertOp]);
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
            notify("Error", "Could not upload media, please try again later");
            console.error(resp);
            clearFileList(fileInput);
            return;
        }

        const body = await resp.json();

        listElt.append(
            $e("a", { href: body.url, textContent: body.url }),
            $e("button", {
                onclick: () =>
                    insertTextAtCursor(
                        window.model.activeEditor,
                        `![Enter alternate description here](${body.url})`,
                    ),
                textContent: "Insert markdown",
            }),
            $e("button", {
                onclick: () => listElt.remove(),
                textContent: "Delete",
            }),
        );
        addFileInput();
    };

    const listElt = $e("li", {}, [
        $e("input", {
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

function switchTo(targetButton) {
    const targetTab = window.model.tabs.get(targetButton);
    if (!targetTab.classList.contains("hidden")) {
        return false;
    }
    targetButton.classList.add("active");

    targetTab.classList.remove("hidden");

    for (const [button, tab] of window.model.tabs) {
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
                    markdown: window.model.editor.getValue(),
                }),
            });

            if (response.status === 200) {
                const json = await response.json();
                article.innerHTML = json.rendered;
            }
        }
    });
