import { execFileSync } from 'node:child_process';
import { joinNodePath } from './paths';
import type { CommandResult, NodeTarget, ServiceState } from './index';

function quote(value: string): string {
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

export class DockerNode implements NodeTarget {
  readonly os = 'linux' as const;

  constructor(
    private readonly container: string,
    readonly host: string,
    readonly workPath: string,
  ) {}

  join(relative: string): string {
    return joinNodePath(this.workPath, relative);
  }

  shell(script: string): CommandResult {
    try {
      const stdout = execFileSync(
        'docker',
        ['exec', this.container, 'sh', '-c', script],
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
    const result = this.shell(`cat ${quote(this.join(relative))}`);
    if (result.code !== 0) {
      throw new Error(`cannot read ${relative} on the node: ${result.stderr}`);
    }

    return result.stdout;
  }

  exists(relative: string): boolean {
    return this.shell(`test -e ${quote(this.join(relative))}`).code === 0;
  }

  listDir(relative: string): string[] {
    const result = this.shell(`ls -1 ${quote(this.join(relative))}`);
    if (result.code !== 0) {
      return [];
    }

    return result.stdout.split('\n').filter((line) => line.length > 0);
  }

  mode(relative: string): string {
    const result = this.shell(`stat -c %a ${quote(this.join(relative))}`);
    if (result.code !== 0) {
      throw new Error(`cannot stat ${relative} on the node: ${result.stderr}`);
    }

    return result.stdout.trim();
  }

  serviceState(name: string): ServiceState {
    const unit = `${name}.service`;
    if (this.shell(`systemctl list-unit-files ${quote(unit)} | grep -q ${quote(unit)}`).code !== 0) {
      return 'absent';
    }

    return this.shell(`systemctl is-active --quiet ${quote(unit)}`).code === 0
      ? 'running'
      : 'stopped';
  }

  filesBinaryVersion(): string {
    const result = this.shell('/usr/local/bin/gameap-files version');
    if (result.code !== 0) {
      throw new Error(`gameap-files version failed: ${result.stderr || result.stdout}`);
    }

    return result.stdout.trim();
  }

  listeningPorts(): number[] {
    const result = this.shell('ss -lnt');

    return [...result.stdout.matchAll(/:(\d+)\s/g)]
      .map((match) => Number(match[1]))
      .filter((port, index, all) => all.indexOf(port) === index);
  }
}
