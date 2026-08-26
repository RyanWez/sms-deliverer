export function portNum(name: string): number {
  const m = name.match(/(\d+)$/);
  return m ? parseInt(m[1], 10) : 0;
}

export function portLabel(name: string): string {
  return `Port ${portNum(name)}`;
}
