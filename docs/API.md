# API Reference

Base URL: `http://localhost:8007`.

## Health

```bash
curl localhost:8007/health
```

```json
{ "status": "ok", "version": "0.1.0" }
```

## Priority queue

```bash
curl localhost:8007/api/v1/queue
```

```json
{
  "queue": [ {
    "id": "rev-ada-labs", "patient_id": "ada",
    "risk_score": 45.0,
    "rationale": "HBA1C 7.2 % high (bound 4–5.7); LDL 142 mg/dL high (bound 0–130)",
    "state": "pending", "created_at": "2026-10-05T16:00:00Z" } ],
  "pending": 3, "total": 3
}
```

Risk-sorted (desc), recency tiebreak. The healthy control patient never
appears.

## 360 view

```bash
curl localhost:8007/api/v1/patients/grace/360
```

```json
{
  "patient_id": "grace",
  "sources": {
    "vitals":   [ { "code": "SYS_BP", "label": "Systolic BP",
                    "value": 152, "unit": "mmHg", "taken_at": "2026-09-02 09:00:00" } ],
    "wearables": [ { "code": "HRV_MS", "label": "HRV (nightly)",
                     "value": 14, "unit": "ms", "taken_at": "…" } ]
  },
  "review_items": [ { "id": "rev-grace-vitals", "risk_score": 15, "…": "…" } ]
}
```

## Synthesis (stub)

```bash
curl -X POST localhost:8007/api/v1/patients/ada/synthesis \
  -H 'content-type: application/json' -d '{}'
```

```json
{
  "patient_id": "ada", "review_id": "rev-ada-labs",
  "synthesis": "Patient ada has 2 review-worthy signal(s) from labs: HBA1C …",
  "risk_score": 45, "model": "stub",
  "disclaimer": "Demo synthesis from synthetic data. Not medical advice."
}
```

## Review workflow

```bash
curl -X POST localhost:8007/api/v1/reviews/rev-ada-labs \
  -H 'content-type: application/json' \
  -d '{"action":"acknowledge"}'      # or "escalate"
```

```json
{ "id": "rev-ada-labs", "patient_id": "ada", "state": "acknowledged",
  "risk_score": 45 }
```

Acknowledged/escalated items disappear from the queue.

## Errors

| Status | Meaning |
|--------|---------|
| `400` | unknown review action |
| `404` | unknown patient / review / no items for synthesis |
