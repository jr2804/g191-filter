// Initialize MathJax to render arithmatex spans.
// The pymdownx.arithmatex extension emits <span class="arithmatex">\(...\)</span>.
// zensical loads mathjax@3 via CDN but does not call typeset itself, so we do.
(function () {
  function ready() {
    return typeof window.MathJax !== "undefined" && typeof window.MathJax.typesetPromise === "function";
  }
  function run() {
    if (ready()) {
      window.MathJax.typesetPromise(document.querySelectorAll(".arithmatex")).catch(function (err) {
        console.error("MathJax typeset error:", err);
      });
    } else {
      setTimeout(run, 50);
    }
  }
  run();
})();
