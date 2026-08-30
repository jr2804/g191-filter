// Initialize mermaid.js for client-side diagram rendering.
// mermaid.min.js loads asynchronously from a CDN and starts before DOMContentLoaded,
// so we cannot rely on window.load / DOMContentLoaded timing (the CDN script may
// still be executing while DOMContentLoaded has already fired). Instead, poll for
// the global to be ready, then initialize and render all `.mermaid` blocks.
(function () {
  function ready() {
    return typeof window.mermaid !== "undefined" && typeof window.mermaid.initialize === "function";
  }

  function run() {
    if (!ready()) {
      setTimeout(run, 50);
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
    window.mermaid.run();
  }

  run();
})();
