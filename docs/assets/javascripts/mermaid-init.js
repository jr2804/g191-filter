// Initialize mermaid.js for client-side diagram rendering.
// mermaid.min.js sets startOnLoad=false via this init before DOMContentLoaded,
// then we render explicitly after both scripts have executed.
(function () {
  const dark = document.documentElement.getAttribute("data-md-color-scheme") === "slate";
  mermaid.initialize({
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
  document.addEventListener("DOMContentLoaded", () => mermaid.run());
})();
