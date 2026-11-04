/** Fades out the HTML loading screen from index.html once the app is on screen. */
export function hideSplash() {
  const splash = document.getElementById("vf-splash");
  if (!splash || splash.classList.contains("vf-splash--done")) return;
  // Wait one frame so the first real paint is underneath before fading.
  requestAnimationFrame(() => {
    splash.classList.add("vf-splash--done");
    window.setTimeout(() => splash.remove(), 400);
  });
}
