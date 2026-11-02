import {test,expect} from '@playwright/test';
import {switchLanguage} from './helpers';

for (const view of ['simple','detailed']) {
  test(`${view} exposes RAW aggregate and preserves the exact choice across views and language`,async({page})=>{
    await page.goto(`/tests/fixture.html?view=${view}&codecs=raw`);
    await page.getByLabel('素材来源',{exact:true}).fill('/test/source');
    await page.getByLabel('目标文件夹',{exact:true}).fill('/test/Camera');
    await page.getByRole('button',{name:'扫描素材',exact:true}).click();
    if(view==='simple')await page.getByText('更多选项',{exact:true}).click();
    const codec=page.getByRole('combobox',{name:'视频编码',exact:true});
    await expect(codec.locator('option')).toHaveText(['全部编码','RAW（全部类型）','Blackmagic_RAW','Canon_RAW','HEVC','ProRes_RAW','RED_RAW']);
    await codec.selectOption('RAW');
    await expect.poll(()=>page.evaluate(()=>(globalThis as any).readCommands().filter((c:any)=>c.command==='selection_summary').at(-1)?.args.options.codec)).toBe('RAW');
    await expect.poll(()=>page.evaluate(()=>(globalThis as any).readSavedSettings()?.options.codec)).toBe('RAW');
    if(view==='simple'){
      await expect(page.getByRole('region',{name:'当前筛选',exact:true})).toContainText('RAW（全部类型）');
      await page.getByRole('button',{name:'详细视图',exact:true}).click();
      await page.locator('nav').getByRole('button',{name:'筛选 02'}).click();
    } else {
      await page.getByRole('button',{name:'简易视图',exact:true}).click();
      await page.getByText('更多选项',{exact:true}).click();
    }
    await expect(page.getByRole('combobox',{name:'视频编码',exact:true})).toHaveValue('RAW');
    await switchLanguage(page,'en');
    const english=page.getByRole('combobox',{name:'Video codec',exact:true});
    await expect(english).toHaveValue('RAW');
    await expect(english.locator('option[value="RAW"]')).toHaveText('RAW (all types)');
    await english.selectOption('Blackmagic_RAW');
    await expect.poll(()=>page.evaluate(()=>(globalThis as any).readCommands().filter((c:any)=>c.command==='selection_summary').at(-1)?.args.options.codec)).toBe('Blackmagic_RAW');
  });
}
