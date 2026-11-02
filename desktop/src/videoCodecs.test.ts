import {describe,expect,it} from 'vitest';
import {videoCodecChoices} from './videoCodecs';

describe('video codec choices',()=>{
  it('offers the RAW family without changing concrete codec names',()=>{
    const codecs=['Blackmagic_RAW','Canon_RAW','HEVC','ProRes_RAW'];
    expect(videoCodecChoices(codecs)).toEqual(['RAW',...codecs]);
    expect(codecs).toEqual(['Blackmagic_RAW','Canon_RAW','HEVC','ProRes_RAW']);
  });
  it('keeps the saved selection visible before and after scanning',()=>{
    expect(videoCodecChoices([], 'RED_RAW')).toEqual(['RAW','RED_RAW']);
    expect(videoCodecChoices(['HEVC'],'RAW')).toEqual(['RAW','HEVC']);
    expect(videoCodecChoices(['Canon_RAW','Canon_RAW'],'Canon_RAW')).toEqual(['RAW','Canon_RAW']);
  });
});
