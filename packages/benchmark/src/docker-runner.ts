import { execSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Scenario } from './scenarios.js';

export interface StatSample {
  cpuPercent: number;
  memoryMb: number;
}

export interface ScenarioResult {
  scenario: Scenario;
  cpuPercent: { min: number; avg: number; max: number };
  memoryMb: { min: number; avg: number; max: number };
  samples: number;
  error?: string;
}

export function buildImage(
  contextPath: string,
  dockerfilePath: string,
  tag: string,
): void {
  process.stderr.write(`Building Docker image ${tag}...\n`);
  execSync(`docker build -f "${dockerfilePath}" -t "${tag}" "${contextPath}"`, {
    stdio: ['ignore', 'pipe', process.stderr],
  });
  process.stderr.write(`Image ${tag} built successfully.\n`);
}

export function getImageSize(tag: string): string {
  try {
    const raw = execSync(`docker image inspect "${tag}" --format "{{.Size}}"`, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
    const bytes = parseInt(raw, 10);
    if (Number.isNaN(bytes)) return 'unknown';
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  } catch {
    return 'unknown';
  }
}

function parseCpuPercent(raw: string): number | null {
  const cleaned = raw.trim().replace('%', '');
  if (cleaned === 'N/A' || cleaned === '--' || cleaned === '') return null;
  const val = parseFloat(cleaned);
  return Number.isNaN(val) ? null : val;
}

function parseMemoryMb(raw: string): number | null {
  // Format: "12.5MiB / 7.67GiB" — take first part only
  const part = raw.trim().split('/')[0].trim();
  if (!part || part === 'N/A' || part === '--') return null;

  const match = part.match(/^([\d.]+)\s*(MiB|GiB|MB|GB|KiB|KB|B)$/i);
  if (!match) return null;

  const value = parseFloat(match[1]);
  const unit = match[2].toLowerCase();

  switch (unit) {
    case 'gib':
    case 'gb':
      return value * 1024;
    case 'mib':
    case 'mb':
      return value;
    case 'kib':
    case 'kb':
      return value / 1024;
    case 'b':
      return value / (1024 * 1024);
    default:
      return null;
  }
}

function aggregate(values: number[]): {
  min: number;
  avg: number;
  max: number;
} {
  if (values.length === 0) return { min: 0, avg: 0, max: 0 };
  const min = Math.min(...values);
  const max = Math.max(...values);
  const avg = values.reduce((a, b) => a + b, 0) / values.length;
  return {
    min: parseFloat(min.toFixed(2)),
    avg: parseFloat(avg.toFixed(2)),
    max: parseFloat(max.toFixed(2)),
  };
}

function isContainerRunning(containerName: string): boolean {
  try {
    const result = execSync(
      `docker inspect --format "{{.State.Running}}" "${containerName}" 2>/dev/null`,
      { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] },
    ).trim();
    return result === 'true';
  } catch {
    return false;
  }
}

function collectOneSample(containerName: string): StatSample | null {
  try {
    const raw = execSync(
      `docker stats --no-stream --format "{{.CPUPerc}}\t{{.MemUsage}}" "${containerName}"`,
      { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 5000 },
    ).trim();

    const [cpuRaw, memRaw] = raw.split('\t');
    const cpu = parseCpuPercent(cpuRaw ?? '');
    const mem = parseMemoryMb(memRaw ?? '');

    if (cpu === null || mem === null) return null;
    return { cpuPercent: cpu, memoryMb: mem };
  } catch {
    return null;
  }
}

export async function runBenchmarkScenario(
  scenario: Scenario,
  configYaml: string,
  durationSeconds: number,
): Promise<ScenarioResult> {
  const timestamp = Date.now();
  const containerName = `upslim-bench-${timestamp}`;
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'upslim-bench-'));
  const configPath = path.join(tempDir, 'bench.yaml');

  const cpuSamples: number[] = [];
  const memSamples: number[] = [];
  let containerStarted = false;
  let errorMessage: string | undefined;

  try {
    fs.writeFileSync(configPath, configYaml, 'utf8');

    process.stderr.write(
      `  Starting container for scenario "${scenario.name}" (${scenario.monitors} monitors, ${durationSeconds}s)...\n`,
    );

    const dockerArgs = [
      'run',
      '--rm',
      '-d',
      '--add-host=host.docker.internal:host-gateway',
      '-v',
      `${tempDir}:/config:ro`,
      '--name',
      containerName,
      'upslim-benchmark:latest',
    ];

    execSync(`docker ${dockerArgs.join(' ')}`, {
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    containerStarted = true;

    // Allow a brief startup window before sampling
    await sleep(2000);

    const intervalMs = 2000;
    const iterations = Math.floor((durationSeconds * 1000) / intervalMs);

    for (let i = 0; i < iterations; i++) {
      if (!isContainerRunning(containerName)) {
        process.stderr.write(`  Container exited early at sample ${i + 1}.\n`);
        errorMessage = 'Container exited before benchmark completed';
        break;
      }

      const sample = collectOneSample(containerName);
      if (sample !== null) {
        cpuSamples.push(sample.cpuPercent);
        memSamples.push(sample.memoryMb);
        process.stderr.write(
          `  [${scenario.name}] sample ${i + 1}/${iterations}: CPU=${sample.cpuPercent.toFixed(2)}% MEM=${sample.memoryMb.toFixed(1)}MB\n`,
        );
      }

      await sleep(intervalMs);
    }
  } catch (err) {
    errorMessage = err instanceof Error ? err.message : String(err);
    process.stderr.write(
      `  Scenario "${scenario.name}" error: ${errorMessage}\n`,
    );
  } finally {
    if (containerStarted) {
      try {
        execSync(`docker kill "${containerName}"`, {
          stdio: ['ignore', 'ignore', 'ignore'],
        });
      } catch {
        // Container may have already exited
      }
    }

    try {
      fs.rmSync(tempDir, { recursive: true, force: true });
    } catch {
      // Best effort cleanup
    }
  }

  return {
    scenario,
    cpuPercent: aggregate(cpuSamples),
    memoryMb: aggregate(memSamples),
    samples: cpuSamples.length,
    ...(errorMessage ? { error: errorMessage } : {}),
  };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
