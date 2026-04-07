export interface Scenario {
  name: string;
  monitors: number;
  interval: string;
  durationSeconds: number;
}

export const SCENARIOS: Scenario[] = [
  { name: 'idle', monitors: 1, interval: '30s', durationSeconds: 30 },
  { name: 'light', monitors: 10, interval: '10s', durationSeconds: 60 },
  { name: 'medium', monitors: 50, interval: '5s', durationSeconds: 90 },
  { name: 'heavy', monitors: 100, interval: '5s', durationSeconds: 120 },
];

export const QUICK_SCENARIOS: Scenario[] = [
  { name: 'idle', monitors: 1, interval: '5s', durationSeconds: 15 },
  { name: 'light', monitors: 10, interval: '5s', durationSeconds: 20 },
  { name: 'medium', monitors: 50, interval: '5s', durationSeconds: 30 },
];
