import type { ModelInfo } from '@synthesize/shared-types';

export type CuratedModel = ModelInfo & {
  registryId: string;
  downloadUrl?: string;
  notes?: string;
};

export function recommendQuantization(totalRamGb: number, vramGb: number): string {
  if (vramGb >= 24 || totalRamGb >= 64) return 'Q5_K_M or Q6_K';
  if (vramGb >= 12 || totalRamGb >= 32) return 'Q4_K_M';
  return 'Q3_K_M or smaller model';
}
