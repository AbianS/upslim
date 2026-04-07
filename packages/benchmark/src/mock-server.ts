import http from 'node:http';

export interface MockServer {
  server: http.Server;
  port: number;
  url: string;
}

export function startMockServer(port: number): Promise<MockServer> {
  return new Promise((resolve, reject) => {
    const server = http.createServer(
      (_req: http.IncomingMessage, res: http.ServerResponse) => {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'ok' }));
      },
    );

    server.once('error', reject);

    server.listen(port, '0.0.0.0', () => {
      const addr = server.address();
      const actualPort =
        typeof addr === 'object' && addr !== null ? addr.port : port;
      resolve({
        server,
        port: actualPort,
        url: `http://localhost:${actualPort}`,
      });
    });
  });
}

export function stopMockServer(mockServer: MockServer): Promise<void> {
  return new Promise((resolve, reject) => {
    mockServer.server.close((err: Error | undefined) => {
      if (err) reject(err);
      else resolve();
    });
  });
}
