#!/usr/bin/env python3
"""Build the Sigma scribe help centre (static HTML) from docs/user/.

    python3 scripts/help-build.py <docs/user dir> <out dir>

Needs Python 3 and the `markdown` package (pip install markdown==3.7).

Input layout: <docs/user>/README.md (the guide's home) and twelve folders
NN-slug/, each with index.md (the section) and one .md file per article. Every
file starts with a front block (title, summary, order) and a "# Title".

Output: index.html, <section>/index.html, <section>/<article>.html,
search.json and img/. Links between .md files become links between the
generated pages, links to other repository files become GitHub URLs, referenced
images are copied into img/. Any dead internal link or missing image is listed
on stderr and the exit status is 1 (the pages are still written).
"""
import html
import json
import os
import posixpath
import re
import shutil
import subprocess
import sys
from html.parser import HTMLParser

try:
    import markdown
except ImportError:
    sys.exit("help-build: the 'markdown' package is missing (pip install markdown==3.7)")

REPO = "https://github.com/armandomartires/sigma-scribe"
BLOB, TREE = REPO + "/blob/main/", REPO + "/tree/main/"
SITE = "https://armandomartires.github.io/sigma-scribe/"
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
FRONT = re.compile(r"\A---\n(.*?)\n---[ \t]*\n", re.S)

errors = []


def err(msg):
    errors.append(msg)


# ---------------------------------------------------------------- source ----

def read_page(path):
    with open(path, encoding="utf-8") as fh:
        text = fh.read().replace("\r\n", "\n")
    meta = {}
    m = FRONT.match(text)
    if m:
        for line in m.group(1).split("\n"):
            k, sep, v = line.partition(":")
            if sep:
                meta[k.strip()] = v.strip().strip("\"'")
        text = text[m.end():]
    h1 = re.search(r"^# +(.+?)\s*$", text, re.M)
    if h1:
        text = text[:h1.start()] + text[h1.end():]
    title = meta.get("title") or (h1.group(1) if h1 else os.path.basename(path))
    try:
        order = int(meta.get("order", "999"))
    except ValueError:
        order = 999
    return {"path": path, "title": title, "summary": meta.get("summary", ""),
            "order": order, "body": text.strip("\n")}


def drop_articles_list(body):
    """The section index lists its articles in Markdown; the generated page
    lists them itself, with summaries."""
    return re.sub(r"^## Articles[ \t]*\n.*?(?=^## |\Z)", "", body, flags=re.S | re.M).strip("\n")


def load(udir):
    home = read_page(os.path.join(udir, "README.md"))
    sections = []
    for d in sorted(os.listdir(udir)):
        full = os.path.join(udir, d)
        m = re.match(r"^\d+-(.+)$", d)
        if not (m and os.path.isdir(full) and os.path.isfile(os.path.join(full, "index.md"))):
            continue
        sec = read_page(os.path.join(full, "index.md"))
        sec.update(dir=d, slug=m.group(1), articles=[])
        for f in os.listdir(full):
            if f.endswith(".md") and f != "index.md":
                art = read_page(os.path.join(full, f))
                art.update(slug=f[:-3], section=sec)
                sec["articles"].append(art)
        sec["articles"].sort(key=lambda a: (a["order"], a["slug"]))
        sections.append(sec)
    if not sections:
        sys.exit("help-build: no NN-slug/index.md sections found in " + udir)
    return home, sections


# ------------------------------------------------------------- git, paths ---

def git(cwd, *args):
    try:
        r = subprocess.run(["git", "-C", cwd] + list(args), capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return None
    return r.stdout.strip() if r.returncode == 0 else None


class Site:
    def __init__(self, udir, out, home, sections):
        self.udir, self.out = udir, out
        self.home, self.sections = home, sections
        # the repository holding docs/user; outside git, two levels above it
        self.repo = git(udir, "rev-parse", "--show-toplevel") or os.path.dirname(os.path.dirname(udir))
        self.repo = os.path.realpath(self.repo)
        self.src2out = {os.path.realpath(home["path"]): "index.html"}
        for s in sections:
            self.src2out[os.path.realpath(s["path"])] = "%s/index.html" % s["slug"]
            s["out"] = "%s/index.html" % s["slug"]
            for a in s["articles"]:
                a["out"] = "%s/%s.html" % (s["slug"], a["slug"])
                self.src2out[os.path.realpath(a["path"])] = a["out"]
        self.images = {}

    def repo_rel(self, full):
        return os.path.relpath(os.path.realpath(full), self.repo).replace(os.sep, "/")

    def image(self, full):
        full = os.path.realpath(full)
        if full not in self.images:
            r = self.repo_rel(full)
            r = re.sub(r"^docs/screenshots/", "", r)
            self.images[full] = "img/" + re.sub(r"[^\w.-]+", "_", r)
        return self.images[full]

    def rewrite(self, target, page, kind):
        """Rewrite one href (kind='href') or src (kind='src') found in `page`."""
        t = html.unescape(target).strip()
        if not t or t.startswith("#") or t.startswith("//") or SCHEME.match(t):
            return target
        path, _, frag = t.partition("#")
        path = path.partition("?")[0]
        full = os.path.normpath(os.path.join(os.path.dirname(page["path"]), path))
        here = os.path.dirname(page["out"])
        frag = "#" + frag if frag else ""
        label = "%s: %s" % (self.repo_rel(page["path"]), t)
        if kind == "src":
            if not os.path.isfile(full):
                err("missing image: " + label)
                return target
            return html.escape(posixpath.relpath(self.image(full), here or "."), quote=True)
        real = os.path.realpath(full)
        if real in self.src2out:
            return html.escape(posixpath.relpath(self.src2out[real], here or ".") + frag, quote=True)
        if full.endswith(".md") and os.path.commonpath([real, os.path.realpath(self.udir)]) == os.path.realpath(self.udir):
            err("dead link (no such article): " + label)
            return target
        if not os.path.exists(full):
            err("dead link (no such file): " + label)
            return target
        r = self.repo_rel(full)
        if r.startswith(".."):
            err("link leaves the repository: " + label)
            return target
        return html.escape((TREE if os.path.isdir(full) else BLOB) + r + frag, quote=True)


# ------------------------------------------------------------- rendering ----

class Text(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []

    def handle_data(self, d):
        self.parts.append(d)

    def handle_endtag(self, tag):
        if tag in ("p", "li", "h1", "h2", "h3", "h4", "tr", "pre", "div"):
            self.parts.append(" ")


def plain(htm):
    p = Text()
    p.feed(htm)
    return re.sub(r"\s+", " ", "".join(p.parts)).strip()


def render(site, page):
    body = drop_articles_list(page["body"]) if page.get("articles") is not None else page["body"]
    md = markdown.Markdown(extensions=["tables", "fenced_code", "toc"],
                           extension_configs={"toc": {"toc_depth": "2-3"}})
    htm = md.convert(body)
    heads = []

    def walk(tokens):
        for t in tokens:
            if t["level"] == 2:
                heads.append((t["id"], t["name"]))
            walk(t.get("children", []))
    walk(md.toc_tokens)

    def sub(attr, kind):
        pat = re.compile(r'(\b%s=")([^"]*)(")' % attr)
        return lambda h: pat.sub(lambda m: m.group(1) + site.rewrite(m.group(2), page, kind) + m.group(3), h)

    htm = re.sub(r"<a\b[^>]*>", lambda m: sub("href", "href")(m.group(0)), htm)
    htm = re.sub(r"<img\b[^>]*>", lambda m: sub("src", "src")(m.group(0)).replace("<img ", '<img loading="lazy" ', 1), htm)
    # strip heading permalink noise is not enabled; ids stay on the headings
    return htm, heads


CSS = """
:root {
  --bg: #fafafa; --fg: #151515; --muted: #505050; --rule: #cfcfcf; --panel: #f0f0f0; --link: #151515;
  --serif: "Literata", Georgia, "Times New Roman", serif;
  --sans: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #121212; --fg: #ececec; --muted: #a8a8a8; --rule: #3a3a3a; --panel: #1b1b1b; --link: #ececec;
  }
}
:root[data-theme="dark"] { --bg: #121212; --fg: #ececec; --muted: #a8a8a8; --rule: #3a3a3a; --panel: #1b1b1b; --link: #ececec; }
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body { margin: 0; background: var(--bg); color: var(--fg); font: 17px/1.65 var(--serif); overflow-wrap: anywhere; }
.wrap { max-width: 64rem; margin: 0 auto; padding: 0 16px; }
a { color: var(--link); text-decoration-thickness: 1px; text-underline-offset: 3px; }
a:hover { text-decoration-thickness: 2px; }
a:focus-visible, input:focus-visible, summary:focus-visible { outline: 3px solid var(--fg); outline-offset: 3px; }
.skip { position: absolute; left: -999px; }
.skip:focus { left: 16px; top: 8px; background: var(--bg); padding: .3rem .6rem; z-index: 2; }
header.bar { border-bottom: 1px solid var(--rule); padding: .9rem 0; }
header.bar .wrap { display: flex; flex-wrap: wrap; align-items: center; gap: .6rem 1.5rem; }
.brand { font: 600 1.25rem/1.2 var(--serif); text-decoration: none; }
.brand span { font-weight: 400; color: var(--muted); }
.search { flex: 1 1 14rem; max-width: 26rem; margin-left: auto; display: none; }
.js .search { display: block; }
.search input { width: 100%; font: 1rem var(--sans); padding: .5rem .7rem; color: var(--fg); background: var(--bg); border: 2px solid var(--fg); border-radius: 0; }
#results { display: none; max-width: 46rem; margin: 1.25rem 0 0; padding: 0; list-style: none; }
#results.on { display: block; }
#results li { padding: .7rem 0; border-bottom: 1px solid var(--rule); margin: 0; }
#results a { font-weight: 600; }
#results .where { font: .8rem var(--sans); color: var(--muted); }
#results p { margin: .15rem 0 0; font-size: .95rem; color: var(--muted); }
main { padding: 1.5rem 0 3rem; }
.crumbs { font: .9rem var(--sans); color: var(--muted); margin: 0 0 1.2rem; }
.crumbs a { color: var(--muted); }
.layout { display: grid; grid-template-columns: 16rem minmax(0, 1fr); gap: 0 3rem; align-items: start; }
nav.tree { font: .92rem/1.4 var(--sans); }
nav.tree details.all { border: 0; padding: 0; }
nav.tree summary.browse { display: none; }
nav.tree details { border-bottom: 1px solid var(--rule); padding: .35rem 0; }
nav.tree summary { cursor: pointer; padding: .25rem 0; font-weight: 600; }
nav.tree ul { list-style: none; margin: .2rem 0 .4rem; padding: 0 0 0 .8rem; }
nav.tree li { margin: 0; }
nav.tree li a { display: block; padding: .22rem .5rem; text-decoration: none; color: var(--muted); }
nav.tree li a:hover { color: var(--fg); text-decoration: underline; }
nav.tree li a[aria-current] { color: var(--fg); font-weight: 600; background: var(--panel); border-left: 3px solid var(--fg); }
nav.tree summary a { text-decoration: none; }
article { min-width: 0; max-width: 46rem; }
h1 { font: 600 clamp(2rem, 6vw, 2.7rem)/1.15 var(--serif); margin: 0 0 .6rem; letter-spacing: -0.01em; }
h2 { font: 600 1.45rem/1.25 var(--serif); margin: 2.2rem 0 .7rem; }
h3 { font: 600 1.1rem/1.3 var(--serif); margin: 1.6rem 0 .4rem; }
h4 { font: 600 1rem/1.3 var(--serif); margin: 1.2rem 0 .3rem; }
.lede { font-size: 1.2rem; line-height: 1.5; color: var(--muted); margin: 0 0 1.4rem; }
p, ul, ol, table, pre, blockquote { margin: 0 0 1rem; }
ul, ol { padding-left: 1.3rem; }
li { margin-bottom: .3rem; }
code { font: .88em ui-monospace, Menlo, Consolas, monospace; background: var(--panel); padding: .05em .3em; }
pre { background: var(--panel); padding: .8rem 1rem; overflow-x: auto; border: 1px solid var(--rule); }
pre code { background: none; padding: 0; overflow-wrap: normal; }
blockquote { border-left: 3px solid var(--fg); background: var(--panel); padding: .6rem 1rem; color: var(--muted); }
blockquote > :last-child { margin-bottom: 0; }
table { border-collapse: collapse; display: block; overflow-x: auto; max-width: 100%; font: .92rem/1.45 var(--sans); }
th, td { border: 1px solid var(--rule); padding: .4rem .7rem; text-align: left; vertical-align: top; }
th { background: var(--panel); }
img { max-width: 100%; height: auto; border: 1px solid var(--rule); background: #fff; }
hr { border: 0; border-top: 1px solid var(--rule); margin: 2rem 0; }
.toc { background: var(--panel); border-left: 3px solid var(--fg); padding: .7rem 1rem; margin: 0 0 1.6rem; font: .92rem/1.45 var(--sans); }
.toc strong { display: block; margin-bottom: .3rem; }
.toc ul { margin: 0; list-style: none; padding: 0; }
.toc li { margin: .15rem 0; }
.cards { display: grid; gap: 1rem; grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr)); margin: 1.5rem 0 0; padding: 0; list-style: none; }
.cards > li { margin: 0; border: 1px solid var(--rule); padding: 1rem 1.1rem; background: var(--panel); }
.cards h2 { font-size: 1.15rem; margin: 0 0 .5rem; }
.cards h2 a { text-decoration: none; }
.cards h2 a:hover { text-decoration: underline; }
.cards ul { margin: 0; padding-left: 1.1rem; font: .92rem/1.4 var(--sans); }
.cards li li { margin: .2rem 0; }
.alist { list-style: none; padding: 0; margin: 1.5rem 0 0; }
.alist li { padding: .8rem 0; border-top: 1px solid var(--rule); margin: 0; }
.alist li:last-child { border-bottom: 1px solid var(--rule); }
.alist a { font-weight: 600; }
.alist p { margin: .15rem 0 0; color: var(--muted); font-size: .95rem; }
.meta { font: .85rem var(--sans); color: var(--muted); margin-top: 2rem; padding-top: 1rem; border-top: 1px solid var(--rule); }
.meta a { color: var(--muted); }
.pn { display: flex; flex-wrap: wrap; gap: 1rem; justify-content: space-between; margin-top: 1.5rem; font: .95rem var(--sans); }
.pn a { display: block; padding: .6rem .9rem; border: 2px solid var(--fg); text-decoration: none; max-width: 100%; }
.pn a small { display: block; color: var(--muted); font-size: .75rem; }
.pn .next { margin-left: auto; text-align: right; }
footer { padding: 1.5rem 0 3rem; font: .85rem var(--sans); color: var(--muted); border-top: 1px solid var(--rule); }
footer a { color: var(--muted); }
@media (max-width: 760px) {
  body { font-size: 16.5px; }
  .layout { grid-template-columns: minmax(0, 1fr); gap: 1rem; }
  nav.tree { border-top: 1px solid var(--rule); }
  nav.tree summary.browse { display: list-item; padding: .5rem 0; }
  .search { max-width: none; margin-left: 0; flex-basis: 100%; }
  .pn a { flex: 1 1 100%; }
  .pn .next { margin-left: 0; }
}
"""

SCRIPT = """
(function () {
  document.documentElement.className += ' js';
  var all = document.querySelector('nav.tree details.all');
  if (all && window.innerWidth <= 760) all.removeAttribute('open');
  var box = document.getElementById('q'), out = document.getElementById('results');
  if (!box || !out) return;
  var root = document.body.getAttribute('data-root') || '', docs = null, loading = false;
  function load(cb) {
    if (docs) return cb();
    if (loading) return;
    loading = true;
    var x = new XMLHttpRequest();
    x.open('GET', root + 'search.json');
    x.onload = function () { try { docs = JSON.parse(x.responseText); } catch (e) { docs = []; } cb(); };
    x.onerror = function () { docs = []; cb(); };
    x.send();
  }
  function run() {
    var q = box.value.toLowerCase().trim();
    out.innerHTML = '';
    if (!q) { out.className = ''; return; }
    var words = q.split(/\\s+/), hits = [];
    docs.forEach(function (d) {
      var t = d.title.toLowerCase(), s = d.summary.toLowerCase(), h = d.headings.join(' ').toLowerCase(), b = d.text.toLowerCase(), score = 0;
      for (var i = 0; i < words.length; i++) {
        var w = words[i], p = 0;
        if (t.indexOf(w) >= 0) p += 10;
        if (s.indexOf(w) >= 0) p += 4;
        if (h.indexOf(w) >= 0) p += 3;
        if (b.indexOf(w) >= 0) p += 1;
        if (!p) return;
        score += p;
      }
      hits.push([score, d]);
    });
    hits.sort(function (a, b) { return b[0] - a[0]; });
    out.className = 'on';
    if (!hits.length) { var li = document.createElement('li'); li.textContent = 'No articles match.'; out.appendChild(li); return; }
    hits.slice(0, 12).forEach(function (hh) {
      var d = hh[1], li = document.createElement('li'), a = document.createElement('a'), w = document.createElement('div'), p = document.createElement('p');
      a.href = root + d.url; a.textContent = d.title;
      w.className = 'where'; w.textContent = d.section;
      p.textContent = d.summary;
      li.appendChild(a); li.appendChild(w); li.appendChild(p); out.appendChild(li);
    });
  }
  box.addEventListener('input', function () { load(run); });
  box.addEventListener('focus', function () { load(function () {}); });
})();
"""


def e(s):
    return html.escape(s, quote=True)


def page_html(site, page, title, crumbs, content, tree_html, depth, desc):
    root = "../" * depth
    head = (
        '<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        + "<title>%s</title>\n" % e(title)
        + '<meta name="description" content="%s">\n' % e(desc)
        + '<meta name="color-scheme" content="light dark">\n'
        '<link rel="icon" href="data:image/svg+xml,%3Csvg xmlns=\'http://www.w3.org/2000/svg\' viewBox=\'0 0 64 64\'%3E%3Crect width=\'64\' height=\'64\' rx=\'12\' fill=\'%23111\'/%3E%3Ctext x=\'32\' y=\'47\' font-family=\'Georgia,serif\' font-size=\'44\' text-anchor=\'middle\' fill=\'%23fff\'%3E%CE%A3%3C/text%3E%3C/svg%3E">\n'
        '<link rel="preconnect" href="https://fonts.googleapis.com">\n'
        '<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>\n'
        '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Literata:opsz,wght@7..72,400;7..72,600&amp;display=swap">\n'
        + "<style>" + CSS + "</style>\n</head>\n"
    )
    bar = (
        '<body data-root="%s">\n<a class="skip" href="#main">Skip to content</a>\n'
        '<header class="bar"><div class="wrap">'
        '<a class="brand" href="%s">&Sigma;scribe <span>Help</span></a>'
        '<form class="search" role="search" onsubmit="return false">'
        '<input id="q" type="search" placeholder="Search the help" aria-label="Search the help" autocomplete="off"></form>'
        "</div></header>\n" % (e(root), e(root + "index.html"))
    )
    foot = (
        '<footer><div class="wrap"><a href="%s">Project page</a> &middot; '
        '<a href="%s">Source on GitHub</a></div></footer>\n<script>%s</script>\n</body>\n</html>\n'
        % (SITE, REPO, SCRIPT)
    )
    return head + bar + '<main id="main"><div class="wrap">\n<ul id="results" aria-live="polite"></ul>\n' + crumbs + content + "\n</div></main>\n" + foot


def build(udir, out):
    home, sections = load(udir)
    site = Site(udir, out, home, sections)
    seq = [a for s in sections for a in s["articles"]]
    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(out)
    search = []

    def tree(current, cur_section):
        parts = ['<nav class="tree" aria-label="Guide sections"><details class="all" open><summary class="browse">Browse the guide</summary>']
        for s in sections:
            opened = " open" if s is cur_section else ""
            cur = ' aria-current="page"' if s is current else ""
            parts.append('<details%s><summary><a href="__ROOT__%s"%s>%s</a></summary><ul>' % (opened, e(s["out"]), cur, e(s["title"])))
            for a in s["articles"]:
                cur = ' aria-current="page"' if a is current else ""
                parts.append('<li><a href="__ROOT__%s"%s>%s</a></li>' % (e(a["out"]), cur, e(a["title"])))
            parts.append("</ul></details>")
        parts.append("</details></nav>")
        return "\n".join(parts)

    def write(rel, text):
        p = os.path.join(out, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(text)

    def crumb(*items):
        bits = []
        for label, href in items:
            bits.append('<a href="%s">%s</a>' % (e(href), e(label)) if href else '<span aria-current="page">%s</span>' % e(label))
        return '<p class="crumbs">%s</p>\n' % " &rsaquo; ".join(bits)

    def meta_line(page):
        rel = os.path.relpath(page["path"], udir).replace(os.sep, "/")
        date = git(os.path.dirname(page["path"]), "log", "-1", "--format=%cs", "--", os.path.basename(page["path"]))
        bits = []
        if date:
            bits.append("Last changed %s" % e(date))
        bits.append('<a href="%sdocs/user/%s">Edit this page on GitHub</a>' % (BLOB, e(rel)))
        return '<p class="meta">%s</p>' % " &middot; ".join(bits)

    # home -------------------------------------------------------------
    htm, _ = render(site, dict(home, out="index.html", articles=None))
    cards = []
    for s in sections:
        items = "".join('<li><a href="%s">%s</a></li>' % (e(a["out"]), e(a["title"])) for a in s["articles"])
        cards.append('<li><h2><a href="%s">%s</a></h2><ul>%s</ul></li>' % (e(s["out"]), e(s["title"]), items))
    # the README's own lists duplicate the cards: keep only its opening text
    lead = re.split(r"<h2\b", htm, maxsplit=1)[0]
    content = ('<h1>%s</h1>\n<div class="lede-block">%s</div>\n<ul class="cards">%s</ul>'
               % (e(home["title"]), lead, "\n".join(cards)))
    write("index.html", page_html(site, dict(home, out="index.html"), "Help · Σscribe",
                                  "", content, "", 0, home["summary"] or "Help centre for Σscribe"))
    # sections and articles ----------------------------------------------
    for s in sections:
        s_page = dict(s, out=s["out"])
        htm, _ = render(site, s_page)
        arts = "".join(
            '<li><a href="%s">%s</a><p>%s</p></li>' % (e(a["slug"] + ".html"), e(a["title"]), e(a["summary"]))
            for a in s["articles"])
        main = ('<article><h1>%s</h1>\n%s\n<h2>Articles</h2>\n<ul class="alist">%s</ul>%s</article>'
                % (e(s["title"]), htm, arts, meta_line(s)))
        layout = '<div class="layout">%s\n%s</div>' % (tree(s, s).replace("__ROOT__", "../"), main)
        write(s["out"], page_html(site, s, "%s · Σscribe Help" % s["title"],
                                  crumb(("Help", "../index.html"), (s["title"], None)), layout, "", 1, s["summary"]))
        search.append({"title": s["title"], "summary": s["summary"], "section": s["title"],
                       "headings": [], "text": plain(htm), "url": s["out"]})
        for a in s["articles"]:
            htm, heads = render(site, a)
            i = seq.index(a)
            toc = ""
            if len(heads) >= 2:
                toc = '<nav class="toc" aria-label="On this page"><strong>On this page</strong><ul>%s</ul></nav>' % "".join(
                    '<li><a href="#%s">%s</a></li>' % (e(hid), e(name)) for hid, name in heads)
            pn = []
            if i > 0:
                p = seq[i - 1]
                pn.append('<a class="prev" href="%s" rel="prev"><small>Previous</small>%s</a>' % (
                    e(posixpath.relpath(p["out"], s["slug"])), e(p["title"])))
            if i + 1 < len(seq):
                n = seq[i + 1]
                pn.append('<a class="next" href="%s" rel="next"><small>Next</small>%s</a>' % (
                    e(posixpath.relpath(n["out"], s["slug"])), e(n["title"])))
            main = ('<article><h1>%s</h1>\n<p class="lede">%s</p>\n%s\n%s\n%s\n<nav class="pn" aria-label="Previous and next article">%s</nav></article>'
                    % (e(a["title"]), e(a["summary"]), toc, htm, meta_line(a), "".join(pn)))
            layout = '<div class="layout">%s\n%s</div>' % (tree(a, s).replace("__ROOT__", "../"), main)
            write(a["out"], page_html(site, a, "%s · Σscribe Help" % a["title"],
                                      crumb(("Help", "../index.html"), (s["title"], "index.html"), (a["title"], None)),
                                      layout, "", 1, a["summary"]))
            search.append({"title": a["title"], "summary": a["summary"], "section": s["title"],
                           "headings": [n for _, n in heads], "text": plain(htm), "url": a["out"]})
    write("search.json", json.dumps(search, ensure_ascii=False, separators=(",", ":")))
    for full, rel in site.images.items():
        if os.path.isfile(full):
            os.makedirs(os.path.join(out, "img"), exist_ok=True)
            shutil.copyfile(full, os.path.join(out, rel))
    return len(sections), len(seq), len(site.images)


def main(argv):
    if len(argv) != 3:
        sys.exit("usage: help-build.py <docs/user dir> <out dir>")
    udir, out = os.path.abspath(argv[1]), os.path.abspath(argv[2])
    if not os.path.isfile(os.path.join(udir, "README.md")):
        sys.exit("help-build: %s has no README.md" % udir)
    ns, na, ni = build(udir, out)
    print("help-build: %d sections, %d articles, %d images -> %s" % (ns, na, ni, out))
    if errors:
        print("help-build: %d problem(s):" % len(errors), file=sys.stderr)
        for m in sorted(set(errors)):
            print("  " + m, file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main(sys.argv)
