import {test,expect,type Page} from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import {openSettings,switchLanguage} from './helpers';

async function prepare(page:Page,query=''){
  await page.goto(`/tests/fixture.html?view=simple${query}`);
  await expect(page.getByRole('heading',{name:'导入配置',exact:true})).toBeVisible();
  await page.getByLabel('素材来源',{exact:true}).fill('/test/source');
  await page.getByLabel('目标文件夹',{exact:true}).fill('/test/Camera');
}
async function scan(page:Page){
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await expect(page.getByRole('button',{name:'生成预览',exact:true})).toBeEnabled();
}
async function commands(page:Page,name:string){
  return page.evaluate(command=>(globalThis as any).readCommands().filter((call:{command:string})=>call.command===command),name);
}

test('simple import defaults to three steps, previews real folders and requires warning acknowledgement',async({page})=>{
  await prepare(page,'&rows=mixed');
  await expect(page.locator('nav').getByRole('button')).toHaveCount(3);
  await expect(page.getByRole('button',{name:'简易视图',exact:true})).toHaveAttribute('aria-pressed','true');
  await page.getByRole('combobox',{name:'预设',exact:true}).selectOption('builtin:camera_archive');
  await page.getByLabel('内容名称',{exact:true}).fill('测试街拍');
  await scan(page);
  await expect(page.getByRole('heading',{name:'导入配置',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByRole('heading',{name:'确认导入',exact:true})).toBeVisible();
  const tree=page.getByRole('region',{name:'目标目录',exact:true});
  await expect(tree.getByText('2026.10.06 测试街拍',{exact:true})).toBeVisible();
  await expect(tree.getByText('IMG_0001.CR3',{exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'开始导入',exact:true})).toBeDisabled();
  await page.getByRole('button',{name:'文件',exact:true}).click();
  await expect(page.locator('tbody tr')).toHaveCount(2);
  await page.getByLabel('仅看元数据提示',{exact:true}).check();
  await expect(page.locator('tbody tr')).toHaveCount(1);
  await expect(page.locator('tbody')).toContainText('IMG_0001.CR3');
  expect(await commands(page,'start_import')).toHaveLength(0);
  await page.getByLabel('已确认以上提示').check();
  await page.getByRole('button',{name:'开始导入',exact:true}).click();
  expect(await commands(page,'start_import')).toHaveLength(1);
  await page.evaluate(()=> (globalThis as any).completeImport());
  await expect(page.getByRole('heading',{name:'导入完成',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'打开目标文件夹',exact:true}).click();
  expect((await commands(page,'open_task_destination'))[0].args).toEqual({taskId:'task-1'});
});

test('switching views preserves custom rules, subject, destination and selection',async({page})=>{
  await prepare(page);
  await page.getByRole('combobox',{name:'预设',exact:true}).selectOption('builtin:camera_archive');
  await page.getByLabel('内容名称',{exact:true}).fill('中文 café 📷');
  await page.getByRole('combobox',{name:'拍摄日期',exact:true}).selectOption('custom');
  await page.getByLabel('开始日期',{exact:true}).fill('2026-10-06');
  await page.getByLabel('结束日期',{exact:true}).fill('2026-10-06');
  await scan(page);
  await page.getByRole('button',{name:'详细视图',exact:true}).click();
  await expect(page.locator('nav').getByRole('button')).toHaveCount(5);
  await page.locator('nav').getByRole('button',{name:'预设 03'}).click();
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('中文 café 📷');
  await page.getByLabel('照片文件名（不含扩展名）',{exact:true}).fill('{original}_{seq}');
  await page.getByRole('button',{name:'简易视图',exact:true}).click();
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('中文 café 📷');
  await expect(page.getByLabel('目标文件夹',{exact:true})).toHaveValue('/test/Camera');
  await expect(page.getByRole('region',{name:'当前筛选',exact:true})).toContainText('2026-10-06');
  await expect(page.getByRole('combobox',{name:'预设',exact:true})).toHaveValue('modified');
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByRole('region',{name:'目标目录',exact:true}).getByText('IMG_0001_0001.CR3',{exact:true})).toBeVisible();
  await expect.poll(async()=>page.evaluate(()=>(globalThis as any).readSavedSettings()?.import_view)).toBe('simple');
});

test('saved filters remain visible and resetting them preserves destination, preset and subject',async({page})=>{
  await prepare(page,'&saved=filters');
  const summary=page.getByRole('region',{name:'当前筛选',exact:true});
  await expect(summary).toContainText('HEVC');
  await expect(summary).toContainText('Asia/Tokyo');
  await expect(summary).toContainText('60');
  await page.getByRole('combobox',{name:'预设',exact:true}).selectOption('builtin:camera_archive');
  await page.getByLabel('内容名称',{exact:true}).fill('保留内容');
  await page.getByRole('button',{name:'重置筛选',exact:true}).click();
  await expect(summary).not.toContainText('HEVC');
  await expect(summary).toContainText('Asia/Tokyo');
  await expect(summary).toContainText('+60');
  await expect(page.getByRole('combobox',{name:'预设',exact:true})).toHaveValue('builtin:camera_archive');
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('保留内容');
  await expect(page.getByLabel('目标文件夹',{exact:true})).toHaveValue('/test/Camera');
  await expect(page.getByRole('combobox',{name:'拍摄日期',exact:true})).toHaveValue('all');
});

test('applying a quick preset clears the previous subject and never imports automatically',async({page})=>{
  await prepare(page);
  await page.getByRole('combobox',{name:'预设',exact:true}).selectOption('builtin:camera_archive');
  await page.getByLabel('内容名称',{exact:true}).fill('上次拍摄');
  await page.getByRole('combobox',{name:'预设',exact:true}).selectOption('builtin:date_content');
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('');
  await scan(page);
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByRole('alert')).toContainText('请填写内容名称');
  expect(await commands(page,'start_import')).toHaveLength(0);
});

test('device refresh updates choices without starting a scan',async({page})=>{
  await prepare(page);
  await expect(page.getByRole('button',{name:/NewCard/})).toHaveCount(0);
  await page.getByRole('button',{name:'刷新设备',exact:true}).click();
  await page.getByRole('button',{name:/NewCard/}).click();
  await expect(page.getByLabel('素材来源',{exact:true})).toHaveValue('/test/NewCard');
  expect((await commands(page,'refresh_locations')).length).toBeGreaterThan(0);
  expect(await commands(page,'scan_source')).toHaveLength(0);
});

test('cancelled scan discards preliminary results and a new scan has an independent identity',async({page})=>{
  await prepare(page,'&scan=deferred');
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await page.getByRole('button',{name:'取消扫描',exact:true}).click();
  await expect(page.getByRole('button',{name:'扫描素材',exact:true})).toBeEnabled();
  await expect(page.getByRole('button',{name:'生成预览',exact:true})).toHaveCount(0);
  await expect(page.getByRole('alert')).toHaveCount(0);
  const first=(await commands(page,'scan_source'))[0].args;
  expect((await commands(page,'cancel_scan'))[0].args).toEqual({scanId:first.scanId});
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await expect.poll(async()=> (await commands(page,'scan_source')).length).toBe(2);
  const second=(await commands(page,'scan_source'))[1].args;
  expect(second.scanId).not.toBe(first.scanId);
  await page.evaluate(()=> (globalThis as any).finishScan());
  await expect(page.getByRole('button',{name:'生成预览',exact:true})).toBeEnabled();
});

for(const colorScheme of ['light','dark'] as const)test(`simple view remains accessible in Chinese and English (${colorScheme})`,async({page})=>{
  await page.emulateMedia({colorScheme,reducedMotion:'reduce'});
  await page.setViewportSize({width:1000,height:700});
  await prepare(page);
  for(const language of ['zh','en'] as const){
    if(language==='en')await switchLanguage(page,'en');
    const audit=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
    expect(audit.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))).toEqual([]);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  }
  await page.getByText('More options',{exact:true}).click();
  const expanded=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
  expect(expanded.violations.map(v=>v.id)).toEqual([]);
});

test('disabled verification remains visible in simple configuration, confirmation and task',async({page})=>{
  await prepare(page);
  await openSettings(page);
  await page.getByRole('tab',{name:'性能',exact:true}).click();
  await page.getByLabel('校验文件内容（SHA-256）',{exact:true}).uncheck();
  await page.getByRole('button',{name:'关闭设置',exact:true}).click();
  const banner=page.locator('.workspace > .verification-warning');
  await expect(banner).toBeVisible();
  await scan(page);
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(banner).toBeVisible();
  await page.getByLabel('已确认以上提示').check();
  await page.getByRole('button',{name:'开始导入（不校验）',exact:true}).click();
  await expect(banner).toBeVisible();
  expect((await commands(page,'start_import'))[0].args.verification.enabled).toBe(false);
});


test('metadata-only filtering cannot hide destination blockers or acknowledge a global warning',async({page})=>{
  await prepare(page,'&warnings=global');
  await scan(page);
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  const warning=page.locator('.review-box:not(.blocked)');
  await expect(warning.locator('ul').getByRole('button')).toHaveCount(0);
  await page.getByLabel('仅看元数据提示',{exact:true}).check();
  await expect(page.getByText('目标已存在，不会覆盖 · existing.CR3',{exact:true})).toBeVisible();
  await page.getByLabel('已确认以上提示').check();
  await expect(page.getByRole('button',{name:'开始导入',exact:true})).toBeDisabled();
  expect(await commands(page,'start_import')).toHaveLength(0);
});
