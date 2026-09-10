import { execFileSync } from 'node:child_process';
import { joinNodePath } from './paths';
import type { CommandResult, NodeTarget, ServiceState } from './index';

function quote(value: string): string {
  return `'${value.replace(/'/g, "''")}'`;
}

// The Windows node is the runner itself, so every command runs locally. The
// daemon is installed with --work-path C:\gameap, which makes the daemon's
// tools path C:\gameap\tools — the directory the plugin's Windows version probe
// hard-codes.
export class WindowsNode implements NodeTarget {
  readonly os = 'windows' as const;

  constructor(
    readonly host: string,
    readonly workPath: string,
  ) {}

  join(relative: string): string {
    return joinNodePath(this.workPath, relative);
  }

  shell(script: string): CommandResult {
    try {
      const stdout = execFileSync(
        'powershell',
        ['-NoProfile', '-NonInteractive', '-Command', script],
        { encoding: 'utf8', timeout: 120_000, stdio: ['ignore', 'pipe', 'pipe'] },
      );

      return { code: 0, stdout, stderr: '' };
    } catch (error) {
      const failure = error as { status?: number; stdout?: string; stderr?: string };

      return {
        code: failure.status ?? 1,
        stdout: failure.stdout ?? '',
        stderr: failure.stderr ?? String(error),
      };
    }
  }

  readFile(relative: string): string {
    const result = this.shell(
      `Get-Content -LiteralPath ${quote(this.join(relative))} -Raw`,
    );
    if (result.code !== 0) {
      throw new Error(`cannot read ${relative} on the node: ${result.stderr}`);
    }

    return result.stdout;
  }

  exists(relative: string): boolean {
    const result = this.shell(
      `if (Test-Path -LiteralPath ${quote(this.join(relative))}) { 'yes' } else { 'no' }`,
    );

    return result.stdout.trim() === 'yes';
  }

  listDir(relative: string): string[] {
    const result = this.shell(
      `Get-ChildItem -LiteralPath ${quote(this.join(relative))} -Name -ErrorAction SilentlyContinue`,
    );

    return result.stdout.split(/\r?\n/).filter((line) => line.length > 0);
  }

  // NTFS has no POSIX mode; the specs skip the 0600 assertion here.
  mode(): null {
    return null;
  }

  serviceState(name: string): ServiceState {
    const result = this.shell(
      `$s = Get-Service -Name ${quote(name)} -ErrorAction SilentlyContinue; ` +
        `if ($null -eq $s) { 'absent' } else { $s.Status.ToString() }`,
    );
    const state = result.stdout.trim().toLowerCase();
    if (state === 'absent') {
      return 'absent';
    }

    return state === 'running' ? 'running' : 'stopped';
  }

  filesBinaryVersion(): string {
    const binary = this.join('tools/gameap-files/gameap-files.exe');
    const result = this.shell(`& ${quote(binary)} version`);
    if (result.code !== 0) {
      throw new Error(`gameap-files version failed: ${result.stderr || result.stdout}`);
    }

    return result.stdout.trim();
  }

  listeningPorts(): number[] {
    const result = this.shell(
      "(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue).LocalPort | Sort-Object -Unique",
    );

    return result.stdout
      .split(/\r?\n/)
      .map((line) => Number(line.trim()))
      .filter((port) => Number.isFinite(port) && port > 0);
  }
}
