// Background jobs by module. Kept in a store so a scan keeps running (and its result stays) when
// the user switches to another section.
import { create } from "zustand";
import { api, errorMessage, startJob } from "../lib/ipc";
import type { JobEvent, ScanProgress } from "../types";

export type JobStatus = "idle" | "running" | "done" | "cancelled" | "failed";

export type JobState<T = unknown> = {
  status: JobStatus;
  jobId?: string;
  root?: string;
  progress?: ScanProgress;
  stage?: { stage: string; done: number; total: number };
  warnings: { path: string; message: string }[];
  result?: T;
  error?: string;
  startedAt?: number;
};

type JobsStore = {
  jobs: Record<string, JobState>;
  start: (module: string, command: string, args: Record<string, unknown>) => Promise<void>;
  cancel: (module: string) => Promise<void>;
  update: <T>(module: string, fn: (result: T) => T) => void;
  reset: (module: string) => void;
};

const empty: JobState = { status: "idle", warnings: [] };

export const useJobs = create<JobsStore>((set, get) => ({
  jobs: {},

  start: async (module, command, args) => {
    const current = get().jobs[module];
    if (current?.status === "running") return;
    const patch = (fn: (s: JobState) => Partial<JobState>) =>
      set((st) => ({ jobs: { ...st.jobs, [module]: { ...(st.jobs[module] ?? empty), ...fn(st.jobs[module] ?? empty) } } }));

    set((st) => ({
      jobs: { ...st.jobs, [module]: { status: "running", warnings: [], startedAt: Date.now(), result: undefined } },
    }));
    const onEvent = (e: JobEvent<unknown>) => {
      switch (e.event) {
        case "start":
          patch(() => ({ jobId: e.jobId, root: e.root }));
          break;
        case "progress": {
          const { event: _event, ...progress } = e;
          patch(() => ({ progress }));
          break;
        }
        case "stage":
          patch(() => ({ stage: { stage: e.stage, done: e.done, total: e.total } }));
          break;
        case "warning":
          patch((s) => ({ warnings: s.warnings.length < 200 ? [...s.warnings, { path: e.path, message: e.message }] : s.warnings }));
          break;
        case "complete":
          patch(() => ({ status: "done", result: e.result }));
          break;
        case "cancelled":
          patch(() => ({ status: "cancelled", result: e.result }));
          break;
        case "failed":
          patch(() => ({ status: "failed", error: e.message }));
          break;
      }
    };
    try {
      const jobId = await startJob(command, args, onEvent);
      patch(() => ({ jobId }));
    } catch (err) {
      patch(() => ({ status: "failed", error: errorMessage(err) }));
    }
  },

  cancel: async (module) => {
    const id = get().jobs[module]?.jobId;
    if (id) await api.cancelJob(id);
  },

  update: (module, fn) =>
    set((st) => {
      const job = st.jobs[module];
      if (!job?.result) return st;
      return { jobs: { ...st.jobs, [module]: { ...job, result: fn(job.result as never) } } };
    }),

  reset: (module) => set((st) => ({ jobs: { ...st.jobs, [module]: empty } })),
}));

export function useJob<T>(module: string) {
  const job = useJobs((s) => s.jobs[module]) as JobState<T> | undefined;
  return job ?? (empty as JobState<T>);
}
