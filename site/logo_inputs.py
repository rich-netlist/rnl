"""What the homepage's logo (site/render-logo.py) is drawn from: brand/rnl-logo.sp and the caption that apps/web/build.mjs gives
it for the viewer's interactive logo. The SVGs carry a hash of both, and site/build.py checks it, so that a change of either
without a new rendering fails the build."""
import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def caption(root=ROOT):
    text = (Path(root) / "apps" / "web" / "build.mjs").read_text(encoding="utf-8")
    m = re.search(r"const caption = '((?:[^'\\]|\\.)*)';", text)
    if not m:
        raise ValueError("apps/web/build.mjs: the caption of the interactive logo is not where site/logo_inputs.py looks for it")
    return m.group(1).encode("ascii").decode("unicode_escape")


def inputs_hash(root=ROOT):
    # the text of the source, not its line ends: Git writes CRLF on Windows checkouts and LF on others
    source = (Path(root) / "brand" / "rnl-logo.sp").read_bytes().replace(b"\r\n", b"\n")
    return hashlib.sha256(source + b"\0" + caption(root).encode("utf-8")).hexdigest()[:12]
