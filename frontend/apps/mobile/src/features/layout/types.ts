export interface ResolvedSection {
  type: string;
  mode?: string;
  props?: Record<string, unknown>;
  presentation: 'visible' | 'placeholder';
}

export interface ResolvedScreen {
  screen: string;
  version: number;
  sections: ResolvedSection[];
}
