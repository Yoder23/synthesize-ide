export const STUDIO_TABS: readonly string[];
export function selectProductMode(current: string, requested: string): string;
export function applyBackendSnapshot<T extends object>(state: T, backendSnapshot: unknown): T & { loading: false; error: null; snapshot: unknown };
export function applyPrototypeInteraction(state: Record<string, unknown>, interaction: { action: string; key: string; value?: unknown }): Record<string, unknown>;
export function filterTimeline<T>(events: T[], filter: string): T[];
export function summarizeEvidence(proof: Record<string, unknown> | null | undefined): { complete: number; incomplete: number; verified: boolean };
export function pulseSourceLabel(finding: { experimental?: boolean }): string;
