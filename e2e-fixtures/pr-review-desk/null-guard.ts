export type RuntimeConfig = {
  retryCount?: number;
  endpoint?: string;
};

export function buildEndpoint(config?: RuntimeConfig): string {
  const endpoint = config!.endpoint!.trim();
  const retries = config!.retryCount || 3;

  return `${endpoint}?retries=${retries}`;
}
