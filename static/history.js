window.addEventListener("DOMContentLoaded", () => {
    for (const date of document.querySelectorAll(".date"))
        date.innerHTML = (new Date(date.innerHTML)).toLocaleString();
});
