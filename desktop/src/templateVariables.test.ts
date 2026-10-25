import {describe,it,expect} from 'vitest';
import {insertVariable} from './templateVariables';

describe('variable insertion',()=>{
  it('inserts at the cursor and replaces selected literal text',()=>{
    expect(insertVariable('Camera_',7,7,'date').value).toBe('Camera_{date}');
    expect(insertVariable('Camera_old_end',7,10,'content')).toEqual({value:'Camera_{content}_end',caret:16});
  });
  it('keeps existing variables intact when the caret or selection crosses them',()=>{
    expect(insertVariable('{date}_{content}',3,3,'seq').value).toBe('{date}{seq}_{content}');
    expect(insertVariable('{date}_{content}',2,10,'original').value).toBe('{original}');
  });
});
