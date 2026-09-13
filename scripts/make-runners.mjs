#!/usr/bin/env node
// Builds the generic chatbot launchers from the compiled Langolier binary:
// the same executable, copied under a name that fixes its mode
// (chatbotgui = native window, chatbotweb = server plus browser).
// Used by release.yml to publish one set of launchers per OS. Drop a
// .langolier next to them and it is a chatbot.
//
//   node scripts/make-runners.mjs <binary> <out-dir> [display-name]
import { cpSync, mkdirSync, writeFileSync, chmodSync, existsSync, rmSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { platform } from "node:os";

const [, , binary, outDir, display = "Chatbot"] = process.argv;
if (!binary || !outDir) {
  console.error("usage: make-runners.mjs <binaire> <dossier-sortie> [nom]");
  process.exit(2);
}
const here = dirname(fileURLToPath(import.meta.url));
mkdirSync(outDir, { recursive: true });
const os = platform();

function macApp(name, mode) {
  const app = join(outDir, `${name}.app`);
  rmSync(app, { recursive: true, force: true });
  const macos = join(app, "Contents", "MacOS");
  const res = join(app, "Contents", "Resources");
  mkdirSync(macos, { recursive: true });
  mkdirSync(res, { recursive: true });
  cpSync(binary, join(macos, mode));
  chmodSync(join(macos, mode), 0o755);
  const icon = resolve(here, "..", "src-tauri", "icons", "icon.icns");
  const hasIcon = existsSync(icon);
  if (hasIcon) cpSync(icon, join(res, "icon.icns"));
  writeFileSync(
    join(app, "Contents", "Info.plist"),
    `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>${name}</string>
  <key>CFBundleDisplayName</key><string>${name}</string>
  <key>CFBundleExecutable</key><string>${mode}</string>
  <key>CFBundleIdentifier</key><string>local.langolier.runner.${mode}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  ${hasIcon ? "<key>CFBundleIconFile</key><string>icon</string>" : ""}
</dict></plist>
`,
  );
  return `${name}.app`;
}

const made = [];
if (os === "darwin") {
  made.push(macApp(display, "chatbotgui"), macApp(`${display} Web`, "chatbotweb"));
} else {
  const ext = os === "win32" ? ".exe" : "";
  for (const mode of ["chatbotgui", "chatbotweb"]) {
    const target = join(outDir, `${mode}${ext}`);
    cpSync(binary, target);
    if (os !== "win32") chmodSync(target, 0o755);
    made.push(`${mode}${ext}`);
  }
}
const webBin = os === "darwin" ? `"./${display} Web.app/Contents/MacOS/chatbotweb"` : "./chatbotweb";
writeFileSync(
  join(outDir, "server.sh"),
  `#!/bin/bash\ncd "$(dirname "$0")"\n${webBin} --serve --host 0.0.0.0 --port "\${PORT:-8080}"\n`,
);
chmodSync(join(outDir, "server.sh"), 0o755);
writeFileSync(
  join(outDir, "server.bat"),
  `@echo off\r\ncd /d "%~dp0"\r\nchatbotweb.exe --serve --host 0.0.0.0 --port 8080\r\n`,
);
writeFileSync(
  join(outDir, "README.md"),
  `# Langolier chatbot launchers (${os})\n\nDrop a \`.langolier\` file (exported from Langolier > Assistants) in this folder, then run:\n\n- ${made[0]}: standalone chat window\n- ${made[1]}: chat page in the browser (port 8787)\n- server.sh / server.bat: the page on the network, headless, port 8080\n\nTo update: replace the \`.langolier\`. It is picked up in under 30 seconds, without a restart, keeping conversations.\n`,
);
console.log(`Launchers written to ${outDir}: ${made.join(", ")}`);
