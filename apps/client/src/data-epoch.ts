/** Stable formal-data generation. Never change this for ordinary releases:
 * existing drafts and outbox entries must survive future code updates. */
export const DATA_EPOCH = 'formal-20261101';
export const dataKey = (key: string) => `${DATA_EPOCH}:${key}`;
