// Initialize mermaid.js for client-side diagram rendering.
//
// Three hazards are handled here:
//  1. mermaid.min.js loads asynchronously from a CDN, so window.mermaid may not
//     exist when this script runs — poll until it does.
//  2. With theme feature `navigation.instant`, internal links are fetched and
//     swapped in via XHR, so this script is NOT re-evaluated on navigation.
//     Material exposes `document$`, an observable that emits on every page
//     change (including the first), so subscribe to render each time.
//  3. A rendered diagram keeps its `.mermaid` class and has its children
//     replaced by an <svg>. Re-running on such a node would feed that SVG back
//     to the parser as diagram source, so only unrendered nodes are passed —
//     mermaid.run() would otherwise re-process everything it finds.
(function () {
  function ready() {
    return typeof window.mermaid !== "undefined" && typeof window.mermaid.initialize === "function";
  }

  function render() {
    if (!ready()) {
      setTimeout(render, 50);
      return;
    }
    const dark = document.documentElement.getAttribute("data-md-color-scheme") === "slate";
    window.mermaid.initialize({
      startOnLoad: false,
      theme: "base",
      themeVariables: dark ? {
        background: "#1e2129",
        primaryColor: "#1e2129",
        primaryTextColor: "#d0d5e0",
        lineColor: "#7d8695",
        fontFamily: "Inter, system-ui, sans-serif",
        fontSize: "14px",
      } : {
        background: "#ffffff",
        primaryColor: "#ffffff",
        primaryTextColor: "#27272a",
        lineColor: "#8b949e",
        fontFamily: "Inter, system-ui, sans-serif",
        fontSize: "14px",
      },
      flowchart: { curve: "basis" },
      sequence: { actorFontSize: 14, messageFontSize: 13 },
    });
    const nodes = Array.prototype.filter.call(
      document.querySelectorAll(".mermaid"),
      function (node) { return !node.querySelector("svg"); }
    );
    if (nodes.length) {
      window.mermaid.run({ nodes: nodes });
    }
  }

  if (typeof document$ !== "undefined" && typeof document$.subscribe === "function") {
    document$.subscribe(render);
  } else {
    render();
  }
})();
