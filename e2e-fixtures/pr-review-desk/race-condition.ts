type CacheWriter = {
  write(key: string, value: string): Promise<void>;
};

export async function rememberSelection(writer: CacheWriter, userId: string, repo: string): Promise<string> {
  writer.write(`selection:${userId}`, repo);
  return repo;
}
