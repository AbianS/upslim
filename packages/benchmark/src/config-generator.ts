export function generateConfig(
  monitorCount: number,
  mockServerPort: number,
): string {
  const baseUrl = `http://host.docker.internal:${mockServerPort}/health`;

  const monitors = Array.from({ length: monitorCount }, (_, i) => {
    const index = String(i + 1).padStart(String(monitorCount).length, '0');
    return [
      `  - name: "bench-monitor-${index}"`,
      `    type: http`,
      `    url: "${baseUrl}"`,
      `    conditions:`,
      `      - "[STATUS] == 200"`,
    ].join('\n');
  }).join('\n');

  return [
    'defaults:',
    '  interval: 5s',
    '  timeout: 3s',
    '  failure_threshold: 1',
    '  success_threshold: 1',
    '',
    'monitors:',
    monitors,
    '',
  ].join('\n');
}
