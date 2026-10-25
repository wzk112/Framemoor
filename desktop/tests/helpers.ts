import type {Page} from '@playwright/test';
export async function openSettings(page:Page){await page.getByRole('button',{name:/^(设置|Settings)$/}).click();}
export async function switchLanguage(page:Page,language:'zh'|'en'){
  await openSettings(page);
  await page.getByLabel(/^(界面语言|Interface language)$/).selectOption(language);
  await page.getByRole('button',{name:/^(关闭设置|Close settings)$/}).click();
}
