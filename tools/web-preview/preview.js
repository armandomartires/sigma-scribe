/* Raw WASM loader for the P3-06 static renderer preview. No bundler. */
(function () {
  const FLAG_NIGHT = 1;
  const FLAG_LANDSCAPE = 2;
  const status = document.getElementById("status");
  const canvas = document.getElementById("page");
  const ctx = canvas.getContext("2d");
  let exports = null;

  function setStatus(msg) {
    status.textContent = msg;
  }

  function blit(flags) {
    const rc = exports.preview_render(flags);
    if (rc !== 0) {
      setStatus("render failed: " + rc);
      return;
    }
    const w = exports.preview_width();
    const h = exports.preview_height();
    const ptr = exports.preview_ptr();
    canvas.width = w;
    canvas.height = h;
    const grey = new Uint8Array(exports.memory.buffer, ptr, w * h);
    const img = ctx.createImageData(w, h);
    for (let i = 0, j = 0; i < grey.length; i++, j += 4) {
      const g = grey[i];
      img.data[j] = g;
      img.data[j + 1] = g;
      img.data[j + 2] = g;
      img.data[j + 3] = 255;
    }
    ctx.putImageData(img, 0, 0);
    const night = (flags & FLAG_NIGHT) !== 0;
    const landscape = (flags & FLAG_LANDSCAPE) !== 0;
    setStatus(
      w +
        "×" +
        h +
        " · " +
        (night ? "night" : "day") +
        (landscape ? " · landscape" : " · portrait")
    );
  }

  function flagsFromUi() {
    let f = 0;
    if (document.getElementById("night").checked) f |= FLAG_NIGHT;
    if (document.getElementById("landscape").checked) f |= FLAG_LANDSCAPE;
    return f;
  }

  async function main() {
    setStatus("loading wasm…");
    const resp = await fetch("sigma_web_preview.wasm");
    if (!resp.ok) {
      setStatus(
        "could not fetch sigma_web_preview.wasm. Build it with cargo and copy it next to this page (see README.md)."
      );
      return;
    }
    const { instance } = await WebAssembly.instantiate(await resp.arrayBuffer());
    exports = instance.exports;
    if (exports.preview_abi() !== 1) {
      setStatus("unexpected preview ABI " + exports.preview_abi());
      return;
    }
    document.getElementById("night").addEventListener("change", function () {
      blit(flagsFromUi());
    });
    document.getElementById("landscape").addEventListener("change", function () {
      blit(flagsFromUi());
    });
    blit(0);
  }

  main().catch(function (err) {
    setStatus(String(err));
  });
})();
