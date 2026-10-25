import {test,expect,type Page} from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import {openSettings,switchLanguage} from './helpers';
async function reachPreset(page:Page){await page.goto('/tests/fixture.html');await page.getByLabel('素材来源',{exact:true}).fill('/Volumes/EOS_DIGITAL');await page.getByLabel('目标文件夹',{exact:true}).fill('/test/destination');await page.getByRole('button',{name:'扫描素材'}).click();await expect(page.getByRole('heading',{name:'筛选',exact:true})).toBeVisible();await page.getByRole('button',{name:'继续',exact:true}).click();}
test('Chinese workflow, warning acknowledgement, pause, resume and language switch',async({page})=>{
  const errors:string[]=[];page.on('pageerror',e=>errors.push(String(e)));
  await reachPreset(page);await page.getByRole('button',{name:'生成预览'}).click();
  const start=page.getByRole('button',{name:'开始导入'});await expect(start).toBeDisabled();
  await expect(page.getByText('相机记录的偏移与拍摄地时区不同，请检查夏令时设置。')).toBeVisible();
  await page.getByLabel('已确认以上提示').check();await start.click();
  await page.getByRole('button',{name:'暂停',exact:true}).click();await expect(page.getByRole('heading',{name:'导入已暂停'})).toBeVisible();
  await page.getByRole('button',{name:'继续导入'}).click();await expect(page.getByRole('heading',{name:'导入完成'})).toBeVisible();
  await switchLanguage(page,'en');await expect(page.getByRole('heading',{name:'Import complete'})).toBeVisible();expect(errors).toEqual([]);
});
test('editing preset invalidates approved preview and conflict blocks import',async({page})=>{
  await reachPreset(page);await page.getByRole('button',{name:'生成预览'}).click();await page.getByLabel('已确认以上提示').check();
  await page.locator('footer').getByRole('button',{name:'返回',exact:true}).click();await page.getByLabel('项目名称').fill('CONFLICT');
  await expect(page.locator('nav').getByRole('button',{name:'预览 04'})).toBeDisabled();
  await page.getByRole('button',{name:'生成预览'}).click();await expect(page.getByText('目标已存在，不会覆盖 · existing.CR3')).toBeVisible();
  await page.getByLabel('已确认以上提示').check();await expect(page.getByRole('button',{name:'开始导入'})).toBeDisabled();
});
test('language persists and custom preset can be saved and reapplied',async({page})=>{
  await reachPreset(page);await page.getByLabel('照片文件名（不含扩展名）',{exact:true}).fill('{date}_{seq}');await page.getByLabel('预设名称').fill('My preset');await page.getByRole('button',{name:'保存预设',exact:true}).click();
  await expect(page.locator('.saved-preset').getByText('My preset',{exact:true})).toBeVisible();await switchLanguage(page,'en');await expect(page.getByLabel('Photo filename (without extension)',{exact:true})).toHaveValue('{date}_{seq}');
  await page.getByRole('button',{name:'Build preview'}).click();await expect(page.getByRole('heading',{name:'Import plan'})).toBeVisible();
});

test('filters and presets are editable while metadata loads; preview waits for completion',async({page})=>{
  await page.goto('/tests/fixture.html?scan=deferred');
  await page.getByLabel('素材来源',{exact:true}).fill('/Volumes/EOS_DIGITAL');
  await page.getByLabel('目标文件夹',{exact:true}).fill('/test/destination');
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await expect(page.getByRole('heading',{name:'筛选',exact:true})).toBeVisible();
  await expect(page.getByText('按扩展名初步分类',{exact:false})).toBeVisible();
  await expect(page.getByLabel('视频编码',{exact:true})).toBeDisabled();
  const audit=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
  expect(audit.violations.map(v=>v.id)).toEqual([]);
  await page.getByLabel('开始日期',{exact:true}).fill('2026-10-06');
  await page.getByRole('button',{name:'继续',exact:true}).click();
  await page.getByLabel('项目名称').fill('During scan');
  await expect(page.locator('footer').getByRole('button',{name:'正在扫描元数据',exact:true})).toBeDisabled();
  await expect(page.locator('nav').getByRole('button',{name:'预览 04'})).toBeDisabled();
  await switchLanguage(page,'en');
  await expect(page.locator('footer').getByRole('button',{name:'Reading metadata',exact:true})).toBeDisabled();
  await page.evaluate(()=> (globalThis as unknown as {finishScan:()=>void}).finishScan());
  await expect(page.getByRole('heading',{name:'Folders and naming rules',exact:true})).toBeVisible();
  await expect(page.getByLabel('Project name')).toHaveValue('During scan');
  await page.getByRole('button',{name:'Build preview',exact:true}).click();
  await expect(page.getByText('During scan/2026-10-06/Photo/RAW/20261006_181637_0001_IMG_0001.CR3',{exact:true})).toBeVisible();
});

test('failed background metadata read discards preliminary inventory',async({page})=>{
  await page.goto('/tests/fixture.html?scan=failed');
  await page.getByLabel('素材来源',{exact:true}).fill('/Volumes/EOS_DIGITAL');
  await page.getByLabel('目标文件夹',{exact:true}).fill('/test/destination');
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await expect(page.getByRole('heading',{name:'筛选',exact:true})).toBeVisible();
  await page.evaluate(()=> (globalThis as unknown as {finishScan:()=>void}).finishScan());
  await expect(page.getByRole('heading',{name:'文件夹',exact:true})).toBeVisible();
  await expect(page.getByText('来源文件已变化，请重新扫描。',{exact:true})).toBeVisible();
  await expect(page.locator('nav').getByRole('button',{name:'预设 03'})).toBeDisabled();
  await expect(page.locator('nav').getByRole('button',{name:'预览 04'})).toBeDisabled();
});

test('preset page shows destination folders and updates renamed files before import preview',async({page})=>{
  await reachPreset(page);
  const tree=page.getByRole('region',{name:'目标目录',exact:true});
  await expect(tree.getByText('/test/destination',{exact:true})).toBeVisible();
  await expect(tree.getByText('My Shoot',{exact:true})).toBeVisible();
  await expect(tree.getByText('2026-10-06',{exact:true})).toBeVisible();
  await expect(tree.getByText('RAW',{exact:true})).toBeVisible();
  await expect(tree.getByText('20261006_181637_0001_IMG_0001.CR3',{exact:true})).toBeVisible();
  await page.getByLabel('项目名称').fill('New project');
  await expect(tree.getByText('New project',{exact:true})).toBeVisible();
  await expect(tree.getByText('My Shoot',{exact:true})).not.toBeVisible();
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByText('New project/2026-10-06/Photo/RAW/20261006_181637_0001_IMG_0001.CR3',{exact:true})).toBeVisible();
});

test('date and subject preset creates a first-level folder below the chosen destination',async({page})=>{
  await reachPreset(page);
  await page.getByLabel('预设',{exact:true}).selectOption('builtin:date_content');
  const tree=page.getByRole('region',{name:'目标目录',exact:true});
  await expect(tree.getByText('请填写内容名称。',{exact:true})).toBeVisible();
  await page.getByLabel('内容名称',{exact:true}).fill('街拍');
  await expect(tree.getByText('2026-10-06_街拍',{exact:true})).toBeVisible();
  await expect(tree.getByText('RAW',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByText('2026-10-06_街拍/Photo/RAW/20261006_181637_0001_IMG_0001.CR3',{exact:true})).toBeVisible();
});

test('verification concurrency selects automatic hardware value and reaches the import unchanged',async({page})=>{
  await page.goto('/tests/fixture.html');
  await openSettings(page);
  await page.getByRole('tab',{name:'性能',exact:true}).click();
  await expect(page.getByRole('option',{name:'自动 (2)',exact:true})).toHaveCount(1);
  await page.getByLabel('校验并发数',{exact:true}).selectOption('4');
  await page.getByRole('button',{name:'关闭设置',exact:true}).click();
  await page.getByLabel('素材来源',{exact:true}).fill('/Volumes/EOS_DIGITAL');
  await page.getByLabel('目标文件夹',{exact:true}).fill('/test/destination');
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await page.getByRole('button',{name:'继续',exact:true}).click();
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await page.getByLabel('已确认以上提示').check();
  await page.getByRole('button',{name:'开始导入',exact:true}).click();
  await expect(page.locator('.import-details').getByText('4',{exact:true})).toBeVisible();
  await switchLanguage(page,'en');
  await expect(page.locator('.import-details').getByText('Verification concurrency',{exact:true})).toBeVisible();
});

test('presets fill editable rules; each folder level and filename can change',async({page})=>{
  await reachPreset(page);
  await page.getByLabel('预设',{exact:true}).selectOption('builtin:camera_archive');
  await page.getByLabel('内容名称',{exact:true}).fill('街拍');
  const tree=page.getByRole('region',{name:'目标目录',exact:true});
  await expect(tree.getByText('2026.10.06 街拍',{exact:true})).toBeVisible();
  await expect(tree.getByText('照片',{exact:true})).toBeVisible();
  await expect(tree.getByText('IMG_0001.CR3',{exact:true})).toBeVisible();
  await page.getByText('日期格式与分类目录名',{exact:true}).click();
  await page.getByLabel('日期格式',{exact:true}).fill('DD-MM-YYYY');
  await page.getByLabel('照片目录名',{exact:true}).fill('原片');
  await page.getByLabel('照片文件名（不含扩展名）',{exact:true}).fill('{original}_{seq}');
  await page.getByRole('button',{name:'上移 层级 2',exact:true}).click();
  await expect(page.getByLabel('层级 1',{exact:true})).toHaveValue('{type}');
  await expect(page.getByLabel('预设',{exact:true})).toHaveValue('modified');
  await expect(tree.getByText('原片',{exact:true})).toBeVisible();
  await expect(tree.getByText('06-10-2026 街拍',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'移除 层级 1',exact:true}).click();
  await expect(tree.getByText('原片',{exact:true})).not.toBeVisible();
  await page.getByLabel('预设名称',{exact:true}).fill('街拍规则');await page.getByRole('button',{name:'保存预设',exact:true}).click();
  await page.getByLabel('预设',{exact:true}).selectOption('builtin:original');
  await page.getByRole('button',{name:'应用',exact:true}).click();
  await expect(page.getByLabel('日期格式',{exact:true})).toHaveValue('DD-MM-YYYY');
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('');
  await page.getByLabel('内容名称',{exact:true}).fill('街拍');
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByText('06-10-2026 街拍/IMG_0001_0001.CR3',{exact:true})).toBeVisible();
});


test('variable pickers insert into the intended folder or filename without typing tokens',async({page})=>{
  await reachPreset(page);
  await page.getByLabel('预设',{exact:true}).selectOption('builtin:camera_archive');
  await page.getByLabel('内容名称',{exact:true}).fill('街拍');
  const level=page.getByLabel('层级 1',{exact:true});
  await level.fill('Camera_');
  await page.getByLabel('插入变量 · 层级 1',{exact:true}).selectOption({label:'日期'});
  await expect(level).toHaveValue('Camera_{date}');
  await expect(level).toBeFocused();
  await level.press('End');await level.press('_');
  await page.getByLabel('插入变量 · 层级 1',{exact:true}).selectOption({label:'内容名称'});
  await expect(level).toHaveValue('Camera_{date}_{content}');
  const photo=page.getByLabel('照片文件名（不含扩展名）',{exact:true});
  await photo.fill('');
  await page.getByLabel('插入变量 · 照片文件名（不含扩展名）',{exact:true}).selectOption({label:'原文件名'});
  await photo.press('_');
  await page.getByLabel('插入变量 · 照片文件名（不含扩展名）',{exact:true}).selectOption({label:'序号'});
  await expect(photo).toHaveValue('{original}_{seq}');
  await expect(page.getByLabel('视频文件名（不含扩展名）',{exact:true})).toHaveValue('{original}');
  const video=page.getByLabel('视频文件名（不含扩展名）',{exact:true});
  await video.fill('');
  await page.getByLabel('插入变量 · 视频文件名（不含扩展名）',{exact:true}).selectOption({label:'日期（紧凑）'});
  await expect(video).toHaveValue('{date_compact}');
  await expect(page.getByLabel('插入变量 · 视频文件名（不含扩展名）').locator('option[value="source_dir"]')).toHaveCount(0);
  const tree=page.getByRole('region',{name:'目标目录',exact:true});
  await expect(tree.getByText('Camera_2026.10.06_街拍',{exact:true})).toBeVisible();
  await expect(tree.getByText('IMG_0001_0001.CR3',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'变量说明',exact:true}).click();
  const guide=page.getByRole('dialog',{name:'变量说明',exact:true});
  await expect(guide).toBeVisible();
  await expect(guide.getByText('原文件名，不含扩展名。文件的扩展名会自动保留。',{exact:true})).toBeVisible();
  const audit=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();expect(audit.violations.map(v=>v.id)).toEqual([]);
  await page.keyboard.press('Escape');await expect(guide).not.toBeVisible();
  await expect(page.getByRole('button',{name:'变量说明',exact:true})).toBeFocused();
  await switchLanguage(page,'en');
  await page.getByRole('button',{name:'Variable guide',exact:true}).click();
  await expect(page.getByRole('dialog',{name:'Variable guide'}).locator('dt').filter({hasText:'Relative source folder'})).toBeVisible();
  await page.getByRole('button',{name:'Close variable guide',exact:true}).click();
});

test('saved presets store rules only and applying any preset clears the subject',async({page})=>{
  await reachPreset(page);
  await page.getByLabel('预设',{exact:true}).selectOption('builtin:date_content');
  await page.getByLabel('内容名称',{exact:true}).fill('旧内容');
  await page.getByLabel('照片文件名（不含扩展名）',{exact:true}).fill('{original}_{seq}');
  await page.getByLabel('预设名称',{exact:true}).fill('日常归档');
  await page.getByRole('button',{name:'保存预设',exact:true}).click();
  await expect.poll(async()=>page.evaluate(()=>(globalThis as any).readSavedSettings()?.presets?.[0]?.name)).toBe('日常归档');
  const preset=await page.evaluate(()=>(globalThis as any).readSavedSettings().presets[0]);
  expect(Object.keys(preset).sort()).toEqual(['name','rules']);expect(JSON.stringify(preset)).not.toContain('旧内容');
  await page.getByLabel('内容名称',{exact:true}).fill('当前内容');
  await page.locator('.saved-preset').getByRole('button',{name:'应用',exact:true}).click();
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('');
  await page.getByLabel('内容名称',{exact:true}).fill('重新填写');
  await page.getByLabel('预设',{exact:true}).selectOption('builtin:camera_archive');
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('');
  await page.getByLabel('内容名称',{exact:true}).fill('新的内容');
  await page.getByLabel('预设',{exact:true}).selectOption('saved:0');
  await expect(page.getByLabel('内容名称',{exact:true})).toHaveValue('');
  await page.getByLabel('内容名称',{exact:true}).fill('这次拍摄');
  await page.getByRole('button',{name:'生成预览',exact:true}).click();
  await expect(page.getByText('2026-10-06_这次拍摄/Photo/RAW/IMG_0001_0001.CR3',{exact:true})).toBeVisible();
});


test('settings shortcut keeps the workflow and persists language and verification options',async({page})=>{
  await reachPreset(page);
  await page.getByLabel('项目名称',{exact:true}).fill('Keep this project');
  await page.keyboard.press('Control+,');
  await expect(page.getByRole('dialog',{name:'设置',exact:true})).toBeVisible();
  await page.getByRole('tab',{name:'性能',exact:true}).click();
  await page.getByLabel('校验并发数',{exact:true}).selectOption('8');
  await page.getByRole('tab',{name:'通用',exact:true}).click();
  await page.getByLabel('界面语言',{exact:true}).selectOption('en');
  await expect(page.getByRole('dialog',{name:'Settings',exact:true})).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading',{name:'Folders and naming rules',exact:true})).toBeVisible();
  await expect(page.getByLabel('Project name',{exact:true})).toHaveValue('Keep this project');
  await expect.poll(async()=>page.evaluate(()=>(globalThis as any).readSavedSettings()?.verification?.readers)).toBe(8);
  expect(await page.evaluate(()=>(globalThis as any).readSavedSettings().language)).toBe('en');
  await openSettings(page);
  await page.getByRole('tab',{name:'Performance',exact:true}).click();
  await expect(page.getByLabel('Verification concurrency',{exact:true})).toHaveValue('8');
  await page.getByRole('button',{name:'Close settings',exact:true}).click();
});

test('settings can open while scanning but verification concurrency waits until it finishes',async({page})=>{
  await page.goto('/tests/fixture.html?scan=deferred');
  await page.getByLabel('素材来源',{exact:true}).fill('/Volumes/EOS_DIGITAL');
  await page.getByLabel('目标文件夹',{exact:true}).fill('/test/destination');
  await expect(page.getByLabel('校验并发数',{exact:true})).not.toBeVisible();
  await page.getByRole('button',{name:'扫描素材',exact:true}).click();
  await openSettings(page);
  await page.getByRole('tab',{name:'性能',exact:true}).click();
  await expect(page.getByLabel('校验并发数',{exact:true})).toBeDisabled();
  await page.getByRole('tab',{name:'通用',exact:true}).click();
  await page.getByLabel('界面语言',{exact:true}).selectOption('en');
  await page.evaluate(()=> (globalThis as unknown as {finishScan:()=>void}).finishScan());
  await page.getByRole('tab',{name:'Performance',exact:true}).click();
  await expect(page.getByLabel('Verification concurrency',{exact:true})).toBeEnabled();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading',{name:'Filters',exact:true})).toBeVisible();
});
