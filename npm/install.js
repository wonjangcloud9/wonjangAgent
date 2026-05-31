// 설치 후 실행(postinstall): 현재 플랫폼에 맞는 사전 빌드 바이너리를
// GitHub Releases에서 내려받아 bin/ 에 저장한다.
//
// 다운로드에 실패해도 npm 설치 자체는 깨지지 않도록(soft-fail) 처리하고,
// 실행 시 launcher가 안내한다.

const fs = require('fs');
const path = require('path');
const pkg = require('./package.json');

const REPO = 'wonjangcloud9/wonjangAgent';

// node 플랫폼/아키텍처 → rust 타깃(릴리스 자산 이름).
const TARGETS = {
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-musl',
  'linux-arm64': 'aarch64-unknown-linux-musl',
  'win32-x64': 'x86_64-pc-windows-msvc',
};

async function main() {
  const key = `${process.platform}-${process.arch}`;
  const target = TARGETS[key];
  const isWin = process.platform === 'win32';
  const binName = isWin ? 'wonjang.exe' : 'wonjang';
  const dest = path.join(__dirname, 'bin', binName);

  if (!target) {
    console.error(
      `[wonjang] 지원하지 않는 플랫폼(${key})입니다. 소스 빌드를 사용하세요:\n` +
        `  cargo install --git https://github.com/${REPO}`
    );
    return; // soft-fail
  }

  const asset = `wonjang-${target}${isWin ? '.exe' : ''}`;
  const url = `https://github.com/${REPO}/releases/download/v${pkg.version}/${asset}`;

  console.log(`[wonjang] 바이너리 내려받는 중...\n  ${url}`);
  try {
    const res = await fetch(url, { redirect: 'follow' });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const buf = Buffer.from(await res.arrayBuffer());
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.writeFileSync(dest, buf);
    if (!isWin) fs.chmodSync(dest, 0o755);
    console.log(`[wonjang] 설치 완료 → ${dest}`);
  } catch (e) {
    console.error(
      `[wonjang] 바이너리 다운로드 실패: ${e.message}\n` +
        `  해당 버전의 릴리스가 아직 없을 수 있습니다.\n` +
        `  소스 빌드: cargo install --git https://github.com/${REPO}`
    );
    // soft-fail: npm 설치를 중단시키지 않는다.
  }
}

main();
