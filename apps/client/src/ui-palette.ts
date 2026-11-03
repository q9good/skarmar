export const neutral={ink:'#23332F',muted:'#566460',border:'#D5DED9',surface:'#FFFFFF',canvas:'#F4F5F2',disabled:'#E4E8E5',disabledInk:'#69756E',danger:'#9A3F37'};
export const palettes={
 home:{ink:'#25624F',soft:'#EAF2ED',page:'#F3F6F2',border:'#BDD0C3',hover:'#215442'},
 goal:{ink:'#25624F',soft:'#EAF2ED',page:'#F3F6F2',border:'#BDD0C3',hover:'#215442'},
 method:{ink:'#4C528A',soft:'#EDEFF8',page:'#F5F5FA',border:'#CCD0E4',hover:'#3D4374'},
 classroom:{ink:'#225E82',soft:'#E5F0F6',page:'#F2F7FA',border:'#BDD2E1',hover:'#1B4E6E'},
 family:{ink:'#8B4B32',soft:'#F5E9DE',page:'#FCF7F2',border:'#DEC3B1',hover:'#733C28'},
 experience:{ink:'#2C684B',soft:'#E7F1E9',page:'#F3F8F3',border:'#B9D0BF',hover:'#22543B'},
 difficulty:{ink:'#7E591B',soft:'#F6EDD5',page:'#FCF8ED',border:'#DCCAA0',hover:'#694916'},
 tasks:{ink:'#5B527E',soft:'#EEEDF5',page:'#F7F6FA',border:'#CFCBDD',hover:'#494265'},
} as const;
export type UiTone=keyof typeof palettes;
export type UiPalette=typeof palettes[UiTone];
export const overlayColors=(tone:UiTone)=>({background:palettes[tone].page,control:neutral.surface,ink:palettes[tone].ink});
