declare module 'yauzl-promise' {
  import type { Readable } from 'node:stream';

  export interface ZipEntry {
    filename: string;
    openReadStream(): Promise<Readable>;
  }

  export interface ZipFile extends AsyncIterable<ZipEntry> {
    close(): Promise<void>;
  }

  export function open(path: string): Promise<ZipFile>;
}
