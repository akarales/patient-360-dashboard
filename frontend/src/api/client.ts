import type {
  PatientEntry,
  QueueEntry,
  SynthesisResult,
  View360,
} from './types';

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api/v1${path}`, {
    ...init,
    headers: init?.body ? { 'Content-Type': 'application/json' } : undefined,
  });
  if (!res.ok) {
    const body = await res.text();
    throw new ApiError(res.status, body || res.statusText);
  }
  return (await res.json()) as T;
}

export function fetchPatients(): Promise<{ patients: PatientEntry[] }> {
  return api('/patients');
}

export function fetchQueue(): Promise<{ queue: QueueEntry[]; pending: number; total: number }> {
  return api('/queue');
}

export function fetch360(patientId: string): Promise<View360> {
  return api(`/patients/${encodeURIComponent(patientId)}/360`);
}

export function requestSynthesis(patientId: string): Promise<SynthesisResult> {
  return api(`/patients/${encodeURIComponent(patientId)}/synthesis`, {
    method: 'POST',
    body: '{}',
  });
}

export function reviewAction(
  reviewId: string,
  action: 'acknowledge' | 'escalate',
): Promise<QueueEntry> {
  return api(`/reviews/${encodeURIComponent(reviewId)}`, {
    method: 'POST',
    body: JSON.stringify({ action }),
  });
}
