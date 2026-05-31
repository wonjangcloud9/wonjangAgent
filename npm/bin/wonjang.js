#!/usr/bin/env node
// 내려받은 네이티브 바이너리로 인자를 그대로 전달하는 런처.

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const isWin = process.platform === 'win32';
const binName = isWin ? 'wonjang.exe' : 'wonjang';
const bin = path.join(__dirname, binName);

if (!fs.existsSync(bin)) {
  console.error(
    '[wonjang] 네이티브 바이너리를 찾을 수 없습니다.\n' +
      '  재설치해 보세요:  npm install -g wonjang-agent\n' +
      '  또는 소스 빌드:   cargo install --git https://github.com/wonjangcloud9/wonjangAgent'
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  console.error(`[wonjang] 실행 실패: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 0);
