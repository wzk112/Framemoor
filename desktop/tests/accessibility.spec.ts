import {test,expect} from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import {openSettings,switchLanguage} from './helpers';
for (const colorScheme of ['light','dark'] as const) test(`Chinese and English screens meet accessibility checks in ${colorScheme} mode`,async({page})=>{
  await page.emulateMedia({colorScheme,reducedMotion:'reduce'});
  await page.setViewportSize({width:1000,height:700});
  await page.goto('/tests/fixture.html');
  for(const language of ['zh','en']){
    if(language==='en')await switchLanguage(page,'en');
    await openSettings(page);
    const settingsAudit=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
    expect(settingsAudit.violations.map(v=>v.id)).toEqual([]);
    await page.keyboard.press('Escape');
    const results=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
    expect(results.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))).toEqual([]);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  }
  await page.getByLabel('Media source',{exact:true}).fill('/Volumes/EOS_DIGITAL');await page.getByLabel('Destination folder',{exact:true}).fill('/test/destination');await page.getByRole('button',{name:'Scan media'}).click();
  for(const button of ['Continue','Build preview']){
    if(button==='Build preview'){
      await page.getByRole('button',{name:'Variable guide',exact:true}).click();
      const help=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
      expect(help.violations.map(v=>v.id)).toEqual([]);
      await page.keyboard.press('Escape');
    }
    const results=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
    expect(results.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))).toEqual([]);
    await page.locator('footer').getByRole('button',{name:button,exact:true}).click();
  }
  const preview=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
  expect(preview.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))).toEqual([]);
  await page.getByLabel('I have reviewed these warnings').check();await page.getByRole('button',{name:'Start import'}).click();
  const importing=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
  expect(importing.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)}))).toEqual([]);
  await page.getByRole('button',{name:'Settings',exact:true}).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog',{name:'Settings',exact:true})).toBeVisible();
  await page.keyboard.press('Tab');expect(await page.evaluate(()=>document.activeElement?.tagName!=='BODY')).toBe(true);
  await page.keyboard.press('Escape');
});
