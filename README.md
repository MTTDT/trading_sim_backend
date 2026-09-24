# Sprendžiamo uždavinio aprašymas

## Sistemos paskirtis

Projekto tikslas – sukurti akcijų pirkimų pardavimų strategijos kūrimo puslapį, sukuriant galimybes naudotojui susirast ir pasistestuoti strategijas prieš pritaikant jas realiom sąlygom.

Veikimo principas - pačią kuriamą platformą sudaro dvi dalys: internetinė aplikacija, kuria naudosis naudotojai, administratorius bei aplikacijų programavimo sąsaja.

Naudotojas norėdamas naudotis platforma, prisiregistruos prie internetinės aplikacijos ir galės sudaryti strategiją, nustatyti indikatorius bei kitus parametrus. Naudotojas galės pasidalinti savo strategijos su kitais naudotojais. Administratorius galės panaikinti, kooreguoti statusą viešai paskelbtom strategijoms.

## Funkciniai reikalavimai

Neregistruotas sistemos naudotojas galės:

- Peržiūrėti reprezentacinį puslapį;
- Prisiungti prie aplikacijos;

Registruotas naudotojas gales:

- Atsiungti nuo internetinės aplikacijos;
- Prisiungti prie platformos;
- Surinkti kolekciją akcijų;
- Sukurti strategiją;
  - Susidielioti parametrus,
  - Pridėti kolekciją akcijų,
  - Patikrinti strategijos veiksmingumą su istoriniais duomenimis
- Paskelbti savo strategiją;
- Peržiūrėti viešai paskelbtas strategijas;
- Nusikopijuoti viešai prieinamą strategiją;

Administratorius gales:

- Patvirtiniti naudotojo registraciją;
- Panaikinti viešą strategiją;
- Šalinti naudotoją;

# Sistemos architektūra

Sistemos sudedamosios dalys:

- Kliento pusė – naudojama React.js (su Vite karkasu)
- Serverio pusė – naudojama Rust (su Axum karkasu)

pav. 1 pavaizduota sistemos diagrama. Sistemos talpinimui yra naudojama Termux serveris. Kiekviena sistemos dalis yra diegiama tame pačiame serveryje. Internetinė aplikacija yra pasiekiama per HHTP protokolą. Šios sistemos veikimui yra reikalinga API, kuris pasiekiamas per aplikacijų programavimo sąsają. Pats API vykdo duomenų mainus su duomenų baze.

<img width="690" height="413" alt="image" src="https://github.com/user-attachments/assets/162eeb30-b83f-4aba-bb6f-01812dff84d9" />


*pav. 1 – sistemos diegimo diagrama*
