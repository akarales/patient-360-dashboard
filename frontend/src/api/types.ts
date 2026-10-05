export type Source = 'labs' | 'vitals' | 'wearables';

export interface Signal {
  code: string;
  label: string;
  value: number;
  unit: string;
  taken_at: string;
}

export interface QueueEntry {
  id: string;
  patient_id: string;
  risk_score: number;
  rationale: string;
  state: 'pending' | 'acknowledged' | 'escalated';
  created_at: string;
}

export interface View360 {
  patient_id: string;
  sources: Partial<Record<Source, Signal[]>>;
  review_items: QueueEntry[];
}

export interface PatientEntry {
  id: string;
  sources: string[];
}

export interface SynthesisResult {
  patient_id: string;
  review_id: string;
  synthesis: string;
  risk_score: number;
  model: string;
  disclaimer: string;
}
