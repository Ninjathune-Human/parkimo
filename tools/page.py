"""Source de la page : le squelette web/app.html, où les styles (web/style.css) et les scripts
(web/js/*.js, dans l'ordre de leur numéro) prennent la place de __STYLE__ et __SCRIPT__."""
import pathlib


def source(root: pathlib.Path) -> str:
    web = root / "web"
    page = (web / "app.html").read_text(encoding="utf-8")
    assert "__STYLE__" in page and "__SCRIPT__" in page
    style = (web / "style.css").read_text(encoding="utf-8").removesuffix("\n")
    script = "\n".join(p.read_text(encoding="utf-8").removesuffix("\n") for p in sorted((web / "js").glob("*.js")))
    return page.replace("__STYLE__", style).replace("__SCRIPT__", script)
