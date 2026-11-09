import type {Observation} from './prd-lessons';
export const observationKinds=['notes','teacher_notes','helpful_notes','progress_notes','interest_notes','difficulty_notes','cause_guess'] as const;
export type ObservationKind=typeof observationKinds[number];
export type ObservationItem={id:string;kind:ObservationKind;content:string};
/** Old paragraphs remain one item; newline is never guessed to mean a new fact. */
export function observationItems(v:Observation):ObservationItem[]{return v.items??observationKinds.flatMap(kind=>v[kind]?.trim()?[{id:'legacy-'+kind,kind,content:v[kind]??''}]:[]);}
export function withObservationItems(v:Observation,items:ObservationItem[]):Observation{
 return {...v,items,...Object.fromEntries(observationKinds.map(kind=>[kind,items.filter(i=>i.kind===kind).map(i=>i.content).join('\n')]))};
}
