// Nord's published base swatches: https://www.nordtheme.com/docs/colors-and-palettes
// Our light UI uses derived shades; the original pastel accents are not used as
// white-text button fills. See docs/ui-palette-research-2026-11-03.md.
export const nord={
 night:'#2E3440',muted:'#4C566A',border:'#D8DEE9',surface:'#E5E9F0',snow:'#ECEFF4',
 teal:'#8FBCBB',ice:'#88C0D0',slate:'#81A1C1',blue:'#5E81AC',
 red:'#BF616A',orange:'#D08770',yellow:'#EBCB8B',green:'#A3BE8C',mauve:'#B48EAD',
} as const;
const mix=(from:string,to:string,amount:number)=>'#'+[1,3,5].map(i=>Math.round(parseInt(from.slice(i,i+2),16)*(1-amount)+parseInt(to.slice(i,i+2),16)*amount).toString(16).padStart(2,'0')).join('').toUpperCase();
export const neutral={ink:nord.night,muted:nord.muted,border:nord.border,surface:'#FFFFFF',canvas:mix(nord.snow,'#FFFFFF',.5),disabled:nord.surface,disabledInk:nord.muted,danger:'#98434D',dangerSoft:mix(nord.red,'#FFFFFF',.87)};
const theme=(accent:string,ink:string)=>({accent,ink,soft:mix(accent,neutral.surface,.84),page:mix(accent,neutral.canvas,.94),border:mix(accent,neutral.border,.65),hover:mix(ink,nord.night,.18),action:mix(accent,neutral.surface,.78)});
export const palettes={
 home:theme(nord.green,'#516A3C'),
 goal:theme(nord.green,'#516A3C'),
 method:theme(nord.mauve,'#805578'),
 classroom:theme(nord.blue,'#47658A'),
 family:theme(nord.teal,'#406A69'),
 experience:theme(nord.green,'#516A3C'),
 difficulty:theme(nord.yellow,'#805D16'),
 tasks:theme(nord.slate,'#436689'),
} as const;
export const coverageScale=[neutral.canvas,palettes.experience.soft,palettes.experience.action,nord.green,palettes.experience.ink];
export const chartColors=[palettes.family.ink,palettes.classroom.ink,palettes.method.ink,palettes.difficulty.ink,palettes.goal.ink,palettes.tasks.ink,'#936047','#376D7C'];
export type UiTone=keyof typeof palettes;
export type UiPalette=typeof palettes[UiTone];
export const overlayColors=(tone:UiTone)=>({background:palettes[tone].page,control:neutral.surface,ink:palettes[tone].ink});
