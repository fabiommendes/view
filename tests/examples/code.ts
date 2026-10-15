// Exemplo de TypeScript: tipos, genéricos e uma função assíncrona.
export interface User {
  id: number;
  name: string;
  email?: string;
}

/** Agrupa usuários pelo primeiro caractere do nome. */
export function groupByInitial(users: User[]): Record<string, User[]> {
  const groups: Record<string, User[]> = {};
  for (const user of users) {
    const initial = user.name.charAt(0).toUpperCase();
    (groups[initial] ??= []).push(user);
  }
  return groups;
}

export async function fetchUsers(url: string): Promise<User[]> {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`falha ao buscar ${url}: ${res.status}`);
  }
  return (await res.json()) as User[];
}

const sample: User[] = [
  { id: 1, name: "Ana", email: "ana@example.com" },
  { id: 2, name: "Ari" },
  { id: 3, name: "Beatriz" },
];

console.log(Object.keys(groupByInitial(sample)));
