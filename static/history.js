window.addEventListener("load", () => {
    for (const date of document.querySelectorAll(".date"))
        date.innerHTML = (new Date(date.innerHTML.trim())).toLocaleString();
});
