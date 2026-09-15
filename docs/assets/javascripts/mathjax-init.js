// Initialize MathJax to render arithmatex spans.
// The pymdownx.arithmatex extension emits <span class="arithmatex">\(...\)</span>.
// zensical loads mathjax@3 via CDN but does not call typeset itself, so we do.
//
// Two timing hazards are handled here:
//  1. The CDN script loads asynchronously, so window.MathJax may not carry
//     typesetPromise when this script runs — poll until it does.
//  2. With theme feature `navigation.instant`, internal links are fetched and
//     swapped in via XHR, so this script is NOT re-evaluated on navigation.
//     Material exposes `document$`, an observable that emits on every page
//     change (including the first), so subscribe to typeset each time.
//     MathJax consumes the spans it processes, so a re-run on an unchanged page
//     finds nothing and is a no-op.
(function () {
  function ready() {
    return typeof window.MathJax !== "undefined" && typeof window.MathJax.typesetPromise === "function";
  }

  function render() {
    if (!ready()) {
      setTimeout(render, 50);
      return;
    }
    var nodes = document.querySelectorAll(".arithmatex");
    if (!nodes.length) {
      return;
    }
    window.MathJax.typesetPromise(nodes).catch(function (err) {
      console.error("MathJax typeset error:", err);
    });
  }

  if (typeof document$ !== "undefined" && typeof document$.subscribe === "function") {
    document$.subscribe(render);
  } else {
    render();
  }
})();
