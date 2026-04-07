import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { generateConfig } from './config-generator.js';
import type { ScenarioResult } from './docker-runner.js';
import {
  buildImage,
  getImageSize,
  runBenchmarkScenario,
} from './docker-runner.js';
import { startMockServer, stopMockServer } from './mock-server.js';
import { generateReport } from './report-generator.js';
import { QUICK_SCENARIOS, SCENARIOS } from './scenarios.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const BENCHMARK_IMAGE_TAG = 'upslim-benchmark:latest';

function parseArgs(): { quick: boolean; skipBuild: boolean } {
  const args = process.argv.slice(2);
  return {
    quick: args.includes('--quick'),
    skipBuild: process.env['SKIP_BUILD'] === '1',
  };
}

function findAvailablePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      const port = typeof addr === 'object' && addr !== null ? addr.port : 0;
      server.close(() => resolve(port));
    });
  });
}

function resolveWorkspaceRoot(): string {
  // Walk up from packages/benchmark/src to the repo root
  return path.resolve(__dirname, '..', '..', '..');
}

async function main(): Promise<void> {
  const { quick, skipBuild } = parseArgs();
  const scenarios = quick ? QUICK_SCENARIOS : SCENARIOS;

  process.stderr.write(`UpSlim Benchmark — ${quick ? 'quick' : 'full'} mode\n`);
  process.stderr.write(
    `Scenarios: ${scenarios.map((s) => s.name).join(', ')}\n\n`,
  );

  const workspaceRoot = resolveWorkspaceRoot();
  const dockerfilePath = path.join(
    workspaceRoot,
    'packages',
    'server',
    'Dockerfile',
  );

  if (!skipBuild) {
    buildImage(workspaceRoot, dockerfilePath, BENCHMARK_IMAGE_TAG);
  } else {
    process.stderr.write('Skipping Docker build (SKIP_BUILD=1).\n');
  }

  const imageSize = getImageSize(BENCHMARK_IMAGE_TAG);
  process.stderr.write(`Image size: ${imageSize}\n\n`);

  const mockPort = await findAvailablePort();
  process.stderr.write(`Starting mock HTTP server on port ${mockPort}...\n`);
  const mockServer = await startMockServer(mockPort);
  process.stderr.write(`Mock server listening at ${mockServer.url}\n\n`);

  const results: ScenarioResult[] = [];

  try {
    for (const scenario of scenarios) {
      process.stderr.write(`\nRunning scenario: ${scenario.name}\n`);
      const configYaml = generateConfig(scenario.monitors, mockPort);
      const result = await runBenchmarkScenario(
        scenario,
        configYaml,
        scenario.durationSeconds,
      );
      results.push(result);
      process.stderr.write(
        `  Done — avg CPU: ${result.cpuPercent.avg.toFixed(2)}%, avg RAM: ${result.memoryMb.avg.toFixed(1)} MB (${result.samples} samples)\n`,
      );
    }
  } finally {
    process.stderr.write('\nStopping mock server...\n');
    await stopMockServer(mockServer);
  }

  const report = generateReport(results, imageSize);

  const resultsDir = path.join(__dirname, '..', 'results');
  fs.mkdirSync(resultsDir, { recursive: true });
  const outputPath = path.join(resultsDir, 'latest.md');
  fs.writeFileSync(outputPath, report, 'utf8');
  process.stderr.write(`\nReport saved to: ${outputPath}\n`);

  process.stdout.write(report);
}

main().catch((err) => {
  process.stderr.write(
    `Fatal error: ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exit(1);
});
