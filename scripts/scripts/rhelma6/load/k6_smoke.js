// k6 smoke test template for Rhelma 6 services
// Usage: k6 run k6_smoke.js

import http from 'k6/http';
import { sleep } from 'k6';

export const options = {
  vus: 5,
  duration: '30s',
};

const BASE = __ENV.RHELMA6_BASE || 'http://127.0.0.1:3000';

export default function () {
  http.get(`${BASE}/healthz`);
  sleep(1);
}
