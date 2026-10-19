import { describe, expect, it } from 'vitest';

import { MEMORY_QUOTA, planIncludesMemory } from './memoryPlan';

describe('planIncludesMemory', () => {
  it('is true for the plans with a memory quota and false otherwise', () => {
    expect(planIncludesMemory('BASIC')).toBe(true);
    expect(planIncludesMemory('PRO')).toBe(true);
    expect(planIncludesMemory('FREE')).toBe(false);
    expect(planIncludesMemory('')).toBe(false);
    // An inherited key is not a plan.
    expect(planIncludesMemory('toString')).toBe(false);
  });

  it('agrees with the quota table it is derived from', () => {
    for (const plan of Object.keys(MEMORY_QUOTA)) expect(planIncludesMemory(plan)).toBe(true);
  });
});
