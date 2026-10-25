import {expect,test} from 'vitest';
import {directoryTree} from './DirectoryTree';
import type {Row} from './types';
test('tree preserves target hierarchy, companion files and duplicate destination rows',()=>{
  const row=(source:string,destination:string)=>({source,destination} as Row);
  const tree=directoryTree([
    row('a.CR3','Shoot/2026-10-25/Photo/RAW/renamed.CR3'),
    row('b.CR3','Shoot/2026-10-25/Photo/RAW/renamed.CR3'),
    row('v.MP4','Shoot/2026-10-25/Video/HEVC/v.MP4'),
    row('v.CPF','Shoot/2026-10-25/Video/HEVC/v.CPF'),
  ]);
  expect(tree.count).toBe(4);
  const day=tree.directories.get('Shoot')!.directories.get('2026-10-25')!;
  expect(day.count).toBe(4);
  const photos=day.directories.get('Photo')!.directories.get('RAW')!;
  expect(photos.files.map(f=>f.source)).toEqual(['a.CR3','b.CR3']);
  expect(day.directories.get('Video')!.directories.get('HEVC')!.files.map(f=>f.source)).toEqual(['v.MP4','v.CPF']);
});
