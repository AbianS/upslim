import os from 'node:os';
import type { ScenarioResult } from './docker-runner.js';

function formatPercent(value: number): string {
  return `${value.toFixed(2)}%`;
}

function formatMb(value: number): string {
  return `${value.toFixed(1)} MB`;
}

function pad(value: string, width: number): string {
  return value.padEnd(width);
}

export function generateReport(
  results: ScenarioResult[],
  imageSize: string,
): string {
  const now = new Date();
  const timestamp = now.toISOString();
  const platform = `${os.type()} ${os.release()} (${os.arch()})`;
  const cpus = os.cpus();
  const cpuModel = cpus.length > 0 ? cpus[0].model.trim() : 'unknown';
  const totalMemGb = (os.totalmem() / 1024 ** 3).toFixed(1);

  const colWidths = {
    scenario: Math.max(8, ...results.map((r) => r.scenario.name.length)),
    monitors: 8,
    interval: 8,
    cpuAvg: 7,
    cpuPeak: 8,
    ramAvg: 7,
    ramPeak: 8,
    samples: 7,
  };

  const header = [
    pad('Scenario', colWidths.scenario),
    pad('Monitors', colWidths.monitors),
    pad('Interval', colWidths.interval),
    pad('CPU avg', colWidths.cpuAvg),
    pad('CPU peak', colWidths.cpuPeak),
    pad('RAM avg', colWidths.ramAvg),
    pad('RAM peak', colWidths.ramPeak),
    pad('Samples', colWidths.samples),
  ].join(' | ');

  const separator = [
    '-'.repeat(colWidths.scenario),
    '-'.repeat(colWidths.monitors),
    '-'.repeat(colWidths.interval),
    '-'.repeat(colWidths.cpuAvg),
    '-'.repeat(colWidths.cpuPeak),
    '-'.repeat(colWidths.ramAvg),
    '-'.repeat(colWidths.ramPeak),
    '-'.repeat(colWidths.samples),
  ].join('-|-');

  const rows = results.map((r) => {
    const note = r.error ? ' *(error)*' : '';
    return [
      pad(r.scenario.name + note, colWidths.scenario),
      pad(String(r.scenario.monitors), colWidths.monitors),
      pad(r.scenario.interval, colWidths.interval),
      pad(formatPercent(r.cpuPercent.avg), colWidths.cpuAvg),
      pad(formatPercent(r.cpuPercent.max), colWidths.cpuPeak),
      pad(formatMb(r.memoryMb.avg), colWidths.ramAvg),
      pad(formatMb(r.memoryMb.max), colWidths.ramPeak),
      pad(String(r.samples), colWidths.samples),
    ].join(' | ');
  });

  const lines = [
    '## UpSlim Resource Benchmark',
    '',
    `> Generated: ${timestamp}`,
    `> Platform: ${platform}`,
    `> CPU: ${cpuModel}`,
    `> Total RAM: ${totalMemGb} GB`,
    `> Docker image size: ${imageSize}`,
    '',
    '### Resource Usage by Scenario',
    '',
    `| ${header} |`,
    `| ${separator} |`,
    ...rows.map((row) => `| ${row} |`),
    '',
    '### Notes',
    '',
    '- CPU % is measured via `docker stats` sampled every 2 seconds.',
    '- RAM includes both RSS and cache as reported by the container runtime.',
    '- Monitors use 5s interval (quick mode) or scenario-specific intervals.',
    '- The mock HTTP server responds instantly from memory — no network latency.',
    '- Measurements begin after a 2-second container startup grace period.',
    '',
  ];

  return lines.join('\n');
}
