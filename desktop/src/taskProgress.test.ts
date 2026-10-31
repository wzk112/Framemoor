import {describe,it,expect} from 'vitest';
import {phaseProgress} from './taskProgress';

const task={status:'running',phase:'preparing',copied:0,total_bytes:100,completed_files:0,total_files:2};
describe('current-stage progress',()=>{
  it('keeps unsized hashing and readback indeterminate, even after all bytes are staged',()=>{
    expect(phaseProgress(task).percent).toBeNull();
    expect(phaseProgress({...task,phase:'verifying',copied:100}).percent).toBeNull();
  });
  it('uses verification counts when available and resets for a new stage',()=>{
    expect(phaseProgress({...task,phase:'final_verifying',verification_done:1,verification_total:2}).percent).toBe(50);
    expect(phaseProgress({...task,phase:'copying',copied:25,verification_done:2,verification_total:0}).percent).toBe(25);
  });
  it('handles empty and inconsistent counts without NaN or premature task completion',()=>{
    expect(phaseProgress({...task,phase:'copying',total_bytes:0}).percent).toBeNull();
    expect(phaseProgress({...task,phase:'publishing',completed_files:5}).percent).toBe(100);
    expect(phaseProgress({...task,status:'complete'}).percent).toBe(100);
  });
});
