export class TezaurError extends Error {}
export interface Hit { n: number; label: string; text: string; page: number; path: string; title: string; cov: number; score: number; peer?: string }
export interface Answer { answer: string; references: string; warnings: string[]; excerpts: { n: number; label: string; page: number; path: string; text: string }[] }
export class Client {
  constructor(url?: string, opts?: { token?: string; workspace?: string });
  health(): Promise<any>; sources(): Promise<any>;
  search(q: string, k?: number): Promise<Hit[]>;
  ask(q: string, mode?: 'ask' | 'compare' | 'gaps', k?: number, lang?: string): Promise<Answer>;
  bib(fmt?: 'csl' | 'ris' | 'bibtex'): Promise<any>;
  mesh_search(q: string, k?: number): Promise<{ hits: Hit[]; peer_hits: Hit[]; peer_errors: string[] }>;
  mesh_pull(peer?: string): Promise<{ added: number; already_held: number; errors: string[] }>;
}
