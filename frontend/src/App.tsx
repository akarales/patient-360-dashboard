import { useCallback, useEffect, useState } from 'react';

import {
  fetch360,
  fetchPatients,
  fetchQueue,
  requestSynthesis,
  reviewAction,
} from '@/api/client';
import type { QueueEntry, SynthesisResult, View360 } from '@/api/types';

const RISK_COLOR = (score: number) =>
  score >= 40 ? 'border-red-300 bg-red-50' : 'border-amber-300 bg-amber-50';

export default function App() {
  const [queue, setQueue] = useState<QueueEntry[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [view, setView] = useState<View360 | null>(null);
  const [synthesis, setSynthesis] = useState<SynthesisResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reloadQueue = useCallback(() => {
    fetchQueue()
      .then((body) => setQueue(body.queue))
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    reloadQueue();
    fetchPatients()
      .then((body) => {
        const ids = body.patients.map((patient) => patient.id);
        setSelected((current) => current ?? ids[0] ?? null);
      })
      .catch((err) => setError(String(err)));
  }, [reloadQueue]);

  useEffect(() => {
    if (!selected) return;
    let cancelled = false;
    fetch360(selected)
      .then((data) => {
        if (!cancelled) setView(data);
      })
      .catch((err) => setError(String(err)));
    setSynthesis(null);
    return () => {
      cancelled = true;
    };
  }, [selected]);

  const act = useCallback(
    async (reviewId: string, action: 'acknowledge' | 'escalate') => {
      try {
        await reviewAction(reviewId, action);
        reloadQueue();
        if (selected) setView(await fetch360(selected));
      } catch (err) {
        setError(String(err));
      }
    },
    [reloadQueue, selected],
  );

  const patientIds = Array.from(new Set(queue.map((item) => item.patient_id)));

  return (
    <div className="flex h-full flex-col">
      <header className="border-b border-line bg-panel px-4 py-3">
        <h1 className="text-base font-semibold">Patient 360 Dashboard</h1>
        <p className="text-xs text-muted">
          Multi-source aggregation · physician priority queue · AI synthesis
          · review workflow — synthetic data, not medical advice
        </p>
      </header>

      <main className="grid flex-1 grid-cols-1 gap-4 overflow-y-auto p-4 lg:grid-cols-2">
        <section className="flex flex-col gap-2">
          <h2 className="text-sm font-semibold">
            Priority queue ({queue.filter((i) => i.state === 'pending').length} pending)
          </h2>
          {queue.map((item) => (
            <article
              key={item.id}
              className={`rounded-lg border p-3 ${RISK_COLOR(item.risk_score)} ${
                item.state !== 'pending' ? 'opacity-60' : ''
              }`}
            >
              <div className="flex items-center justify-between">
                <button
                  type="button"
                  onClick={() => setSelected(item.patient_id)}
                  className="text-sm font-semibold text-primary underline"
                >
                  {item.patient_id}
                </button>
                <span className="rounded bg-white px-2 py-0.5 text-xs font-medium">
                  risk {Math.round(item.risk_score)}
                </span>
              </div>
              <p className="mt-1 text-xs">{item.rationale}</p>
              <p className="mt-1 text-[10px] uppercase tracking-wide text-muted">
                {item.state} · {item.id}
              </p>
              {item.state === 'pending' && (
                <div className="mt-2 flex gap-2">
                  <button
                    type="button"
                    onClick={() => void act(item.id, 'acknowledge')}
                    className="rounded border border-line bg-white px-2 py-1 text-xs"
                  >
                    Acknowledge
                  </button>
                  <button
                    type="button"
                    onClick={() => void act(item.id, 'escalate')}
                    className="rounded bg-red-600 px-2 py-1 text-xs font-medium text-white"
                  >
                    Escalate
                  </button>
                </div>
              )}
            </article>
          ))}
          {queue.length === 0 && (
            <p className="text-xs text-muted">Queue is clear.</p>
          )}
        </section>

        <section className="flex flex-col gap-3">
          <h2 className="text-sm font-semibold">
            {selected ? `${selected} — 360 view` : 'Select a patient'}
          </h2>
          {patientIds.length > 0 && (
            <div className="flex gap-1">
              {patientIds.map((id) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setSelected(id)}
                  className={`rounded border border-line px-2 py-1 text-xs ${
                    selected === id ? 'bg-surface font-semibold' : 'bg-panel'
                  }`}
                >
                  {id}
                </button>
              ))}
            </div>
          )}
          {view &&
            Object.entries(view.sources).map(([source, signals]) => (
              <div
                key={source}
                className="rounded-lg border border-line bg-panel p-3"
              >
                <h3 className="mb-1 text-xs font-semibold uppercase text-muted">
                  {source}
                </h3>
                <ul className="text-xs">
                  {(signals ?? []).map((signal) => (
                    <li key={signal.code}>
                      {signal.label}: {signal.value} {signal.unit}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          {view && view.review_items.length > 0 && (
            <p className="text-xs text-muted">
              {view.review_items.length} review item(s) for this patient.
            </p>
          )}
          {selected && (
            <button
              type="button"
              onClick={() =>
                requestSynthesis(selected)
                  .then(setSynthesis)
                  .catch((err) => setError(String(err)))
              }
              className="w-fit rounded bg-accent px-3 py-1.5 text-xs font-medium text-white"
            >
              AI synthesis (stub)
            </button>
          )}
          {synthesis && (
            <div className="rounded-lg border border-line bg-panel p-3 text-xs">
              <p className="mb-2">{synthesis.synthesis}</p>
              <p className="text-[10px] italic text-muted">
                {synthesis.disclaimer}
              </p>
            </div>
          )}
          {error && (
            <p className="rounded border border-red-300 bg-red-50 p-2 text-xs text-red-800">
              {error}
            </p>
          )}
        </section>
      </main>
    </div>
  );
}
