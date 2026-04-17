use crate::templates::Template;

const BUILTIN_TIMESTAMP: &str = "2026-04-17 00:00:00";

pub fn all() -> Vec<Template> {
    vec![
        soap_visit(),
        hospital_discharge(),
        imaging_report(),
        consultation_letter(),
        surgery_protocol(),
    ]
}

fn builtin(
    id: &str,
    name: &str,
    description: &str,
    content: &str,
    example_input: &str,
    example_output: &str,
) -> Template {
    Template {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        content: content.to_string(),
        example_input: Some(example_input.to_string()),
        example_output: Some(example_output.to_string()),
        is_builtin: true,
        created_at: BUILTIN_TIMESTAMP.to_string(),
        updated_at: BUILTIN_TIMESTAMP.to_string(),
        schema: None,
        system_prompt: None,
    }
}

fn soap_visit() -> Template {
    let content = "Przekształć dyktowanie w ustrukturyzowaną notatkę z wizyty ambulatoryjnej (SOAP).

Struktura wyjściowa:

WYWIAD
  - Dolegliwość główna (1 zdanie)
  - Historia choroby aktualnej (OLD CART — tyle ile lekarz powiedział)
  - Choroby przewlekłe / leki na stałe / alergie

BADANIE PRZEDMIOTOWE
  - Stan ogólny + parametry życiowe (RR, tętno, temperatura)
  - Badania narządowe (tylko wymienione przez lekarza)

ROZPOZNANIE
  - Rozpoznanie główne [ICD-10]
  - Rozpoznania współistniejące

ZALECENIA
  - Farmakoterapia (nazwa, dawka, częstość, czas)
  - Badania dodatkowe
  - Kontrola + red flags (czerwone flagi)";

    let example_input = "Pacjentka czterdzieści pięć lat, zgłasza się z bólem kręgosłupa lędźwiowego od trzech dni. Ból pojawił się po podnoszeniu cięższego przedmiotu, nasila się przy pochylaniu i długim staniu, łagodzi pozycja leżąca. Sama kupiła ketoprofen, pomaga częściowo. Nie promieniuje do nóg, bez zaburzeń czucia. Choroby przewlekłe: nadciśnienie, bierze ramipril pięć miligramów. Alergii nie zgłasza. W badaniu bolesność palpacyjna okolicy lędźwiowej, napięcie mięśni przykręgosłupowych, Laseque ujemny obustronnie. Ciśnienie sto trzydzieści na osiemdziesiąt. Rozpoznaję zespół bólowy kręgosłupa lędźwiowego. Daję naproksen pięćset dwa razy dziennie przez pięć dni, tyzanidyna dwa miligramy wieczorem, unikanie dźwigania, kontrola za tydzień jeśli nie ustąpi. Poinformowałam o konieczności pilnej wizyty przy drętwieniu nóg lub zaburzeniach oddawania moczu.";

    let example_output = "WYWIAD
Dolegliwość główna: ból kręgosłupa L/S od 3 dni.
Początek po podnoszeniu ciężkiego przedmiotu. Ból mechaniczny, nasila się przy pochylaniu i długotrwałej pozycji stojącej, łagodzi pozycja leżąca.
Ketoprofen OTC – z częściową poprawą.
Bez promieniowania do kończyn dolnych, bez zaburzeń czucia.

Choroby przewlekłe: nadciśnienie tętnicze.
Leki na stałe: ramipril 5 mg 1×/d.
Alergie: brak zgłaszanych.

BADANIE PRZEDMIOTOWE
Stan ogólny dobry. RR 130/80 mmHg.
Kręgosłup: bolesność palpacyjna okolicy lędźwiowej, wzmożone napięcie mięśni przykręgosłupowych.
Objaw Lasègue'a ujemny obustronnie.

ROZPOZNANIE
Zespół bólowy kręgosłupa odcinka lędźwiowego [ICD-10: M54.5]
Nadciśnienie tętnicze – kontrolowane [ICD-10: I10]

ZALECENIA
1. Naproksen 500 mg – 1 tabl. 2×/d przez 5 dni (po posiłku).
2. Tyzanidyna 2 mg – 1 tabl. wieczorem.
3. Unikać dźwigania ciężarów i długotrwałej pozycji wymuszonej.
4. Kontrola za 7 dni w razie braku poprawy.
5. Pilna wizyta w razie: drętwienia kończyn dolnych, osłabienia siły mięśniowej, zaburzeń oddawania moczu/stolca.";

    builtin(
        "builtin-soap",
        "Wizyta SOAP",
        "Ambulatoryjna wizyta, struktura wywiad / badanie / rozpoznanie / zalecenia",
        content,
        example_input,
        example_output,
    )
}

fn hospital_discharge() -> Template {
    let content = "Przekształć dyktowaną epikryzę w kartę informacyjną z leczenia szpitalnego zgodną z polskimi standardami dokumentacji medycznej. Procedury wykonane podawaj z kodami ICD-9 (nie ICD-10). Epikryza powinna być narracyjna — jeden logicznie spójny akapit.

Struktura wyjściowa:

ROZPOZNANIE GŁÓWNE [ICD-10]
ROZPOZNANIA WSPÓŁISTNIEJĄCE [ICD-10]
PROCEDURY WYKONANE [ICD-9]

EPIKRYZA
  - Powód hospitalizacji (1-2 zdania)
  - Stan przy przyjęciu
  - Kluczowe badania diagnostyczne
  - Przebieg leczenia
  - Stan przy wypisie

ZALECENIA
  - Farmakoterapia
  - Dieta, aktywność, opieka nad raną
  - Kontrole + skierowania
  - Red flags — kiedy pilnie do SOR

ORZECZENIE O NIEZDOLNOŚCI DO PRACY";

    let example_input = "Pacjent sześćdziesiąt dwa lata, przyjęty w trybie ostrym z SOR z powodu ostrego bólu prawego dolnego kwadranta brzucha od osiemnastu godzin, z nudnościami i stanem podgorączkowym. Przy przyjęciu obrona mięśniowa w prawym dole biodrowym, Blumberg dodatni. Leukocytoza piętnaście tysięcy, CRP dziewięćdziesiąt osiem. Tomografia potwierdziła ostre zapalenie wyrostka bez perforacji. W trybie pilnym appendektomia laparoskopowa, przebieg niepowikłany. Pooperacyjnie ceftriakson i metronidazol przez trzy doby, profilaktyka przeciwzakrzepowa enoksaparyną. Pierwsza doba gorączka do trzydziestu ośmiu i pięć, potem bezgorączkowy, rana goi się prawidłowo. Wypis w stanie dobrym. Ciprofloksacyna pięćset dwa razy dziennie i metronidazol pięćset trzy razy dziennie przez pięć dni, paracetamol w razie bólu, wymiany opatrunku co drugi dzień, zdjęcie szwów za dziesięć dni u lekarza rodzinnego, oszczędzający tryb życia dwa tygodnie, niezdolność do pracy czternaście dni.";

    let example_output = "ROZPOZNANIE GŁÓWNE
Ostre zapalenie wyrostka robaczkowego bez perforacji [ICD-10: K35.80]

PROCEDURY WYKONANE
Appendektomia laparoskopowa [ICD-9: 47.01]

EPIKRYZA
62-letni pacjent przyjęty w trybie ostrym z SOR z powodu ostrego bólu prawego dolnego kwadranta jamy brzusznej trwającego od ok. 18 godzin, z towarzyszącymi nudnościami i stanem podgorączkowym. Przy przyjęciu stwierdzono obronę mięśniową w prawym dole biodrowym, dodatni objaw Blumberga. W badaniach laboratoryjnych: leukocytoza 15 tys./µl, CRP 98 mg/l. W TK jamy brzusznej obraz ostrego zapalenia wyrostka robaczkowego bez cech perforacji. W trybie pilnym wykonano appendektomię laparoskopową – przebieg zabiegu niepowikłany. Pooperacyjnie stosowano ceftriakson i metronidazol przez 3 doby oraz profilaktykę przeciwzakrzepową enoksaparyną. W 1. dobie pooperacyjnej gorączka do 38,5°C, następnie bezgorączkowy. Rana operacyjna goi się prawidłowo, bez cech zapalenia. Wypisywany w stanie ogólnym dobrym.

ZALECENIA
1. Ciprofloksacyna 500 mg – 1 tabl. 2×/d przez 5 dni.
2. Metronidazol 500 mg – 1 tabl. 3×/d przez 5 dni.
3. Paracetamol 500 mg doraźnie przy bólu (max 4×/d).
4. Zmiana opatrunku co 2 dni; szwy do usunięcia w 10. dobie pooperacyjnej u lekarza POZ.
5. Oszczędzający tryb życia przez 2 tygodnie; bez dźwigania >5 kg.
6. Pilne zgłoszenie do SOR w razie: gorączki >38,5°C, nasilającego się bólu brzucha, wymiotów, zaczerwienienia/wycieku z rany.

ORZECZENIE O NIEZDOLNOŚCI DO PRACY
Niezdolny do pracy przez 14 dni od daty wypisu.";

    builtin(
        "builtin-discharge",
        "Karta wypisowa",
        "Karta informacyjna z leczenia szpitalnego (wypis)",
        content,
        example_input,
        example_output,
    )
}

fn imaging_report() -> Template {
    let content = "Przekształć dyktowany opis badania obrazowego w oficjalny opis zgodny ze standardem polskiej radiologii. W sekcji OPIS zachowaj kolejność narządów narzuconą przez typ badania (np. USG jamy brzusznej: wątroba → pęcherzyk → drogi żółciowe → trzustka → śledziona → nerki → pęcherz → aorta → wolny płyn). Wnioski numeruj, bądź zwięzły.

Struktura wyjściowa:

TECHNIKA BADANIA
  - Typ badania, rodzaj głowicy/sekwencji
  - Warunki badania (przygotowanie, kontrast)

PORÓWNANIE
  - Z badaniem z dnia ___ (jeśli podano)

OPIS
  (systematycznie, narząd po narządzie)

WNIOSKI
  1. [numerowane znaleziska]
  2. ...
  3. Sugerowana dalsza diagnostyka (opcjonalnie)";

    let example_input = "USG jamy brzusznej, pacjent przygotowany na czczo. Wątroba niepowiększona, echostruktura wzmożona, rozlana stłuszczenie, ogniskowych zmian nie widzę. Pęcherzyk żółciowy cienkościenny, w świetle dwa złogi średnicy około ośmiu milimetrów, ruchome. Drogi żółciowe wewnątrz- i zewnątrzwątrobowe nieposzerzone, przewód żółciowy wspólny cztery milimetry. Trzustka bez zmian ogniskowych, widoczna w dostępnym zakresie. Śledziona prawidłowa, dziesięć centymetrów. Nerka prawa i lewa bez złogów, bez zastoju, warstwa miąższowa zachowana. Pęcherz wypełniony, gładka ściana. Aorta brzuszna niepowiększona. Wolnego płynu w jamie otrzewnej nie stwierdzam. Wnioski: kamica pęcherzyka żółciowego, stłuszczenie wątroby. Sugeruję konsultację chirurgiczną w kierunku cholecystektomii.";

    let example_output = "USG JAMY BRZUSZNEJ

TECHNIKA
Badanie wykonane sondą convex, pacjent na czczo.

OPIS
Wątroba: niepowiększona, echostruktura wzmożona, rozlana (cechy stłuszczenia). Ognisk patologicznych nie uwidoczniono.
Pęcherzyk żółciowy: ściana cienka, w świetle 2 ruchome złogi o średnicy ok. 8 mm.
Drogi żółciowe: wewnątrz- i zewnątrzwątrobowe nieposzerzone. Przewód żółciowy wspólny 4 mm.
Trzustka: w dostępnym oknie akustycznym bez zmian ogniskowych.
Śledziona: prawidłowej wielkości (10 cm), echostruktura jednorodna.
Nerki: obustronnie bez złogów, bez cech zastoju moczu, warstwa miąższowa zachowana.
Pęcherz moczowy: wypełniony, ściana gładka.
Aorta brzuszna: niepowiększona.
Wolnego płynu w jamie otrzewnej nie stwierdzono.

WNIOSKI
1. Kamica pęcherzyka żółciowego (2 złogi ~8 mm).
2. Stłuszczenie wątroby.
3. Sugerowana konsultacja chirurgiczna w kierunku cholecystektomii.";

    builtin(
        "builtin-imaging",
        "Opis badania obrazowego",
        "Systematyczny opis USG/TK/MR z sekcjami: technika, opis, wnioski",
        content,
        example_input,
        example_output,
    )
}

fn consultation_letter() -> Template {
    let content = "Przekształć dyktowane stanowisko specjalisty w oficjalny list konsultacyjny do lekarza kierującego (zwykle POZ). Zachowuj uprzejmy, profesjonalny ton. Używaj formy grzecznościowej „Szanowna Pani Doktor\" / „Szanowny Panie Doktorze\". Jeżeli dane nagłówka/stopki nie są podane, zostaw placeholdery w nawiasach kwadratowych.

Struktura wyjściowa:

[Nagłówek: specjalizacja, miejscowość, data]

Sz. P. dr [imię i nazwisko lekarza kierującego]
[Miejsce pracy lekarza kierującego]

Dot. pacjenta: [imię i nazwisko], PESEL: [___]

Szanowna Pani Doktor / Szanowny Panie Doktorze,

STRESZCZENIE PROBLEMU (1-2 zdania: kto, z czym przyszedł)
WYWIAD I DOTYCHCZASOWE LECZENIE
WYNIKI BADAŃ (istotne)

ROZPOZNANIE

PROPONOWANE POSTĘPOWANIE (numerowane)

KONTROLA / warunki zakończenia opieki specjalistycznej

Z wyrazami szacunku,
[podpis — autouzupełnienie]";

    let example_input = "List do doktor Nowak. Pacjent pięćdziesiąt osiem lat skierowany z powodu nadciśnienia tętniczego opornego na leczenie dwulekowe, amlodipina pięć i ramipril dziesięć, domowe pomiary sto pięćdziesiąt pięć na dziewięćdziesiąt pięć średnio. Palacz dwadzieścia paczkolat, BMI trzydzieści jeden. W badaniu bez cech niewydolności serca. W ECHO przerost lewej komory, frakcja wyrzutowa sześćdziesiąt procent, bez wad zastawkowych. Holter ciśnieniowy potwierdził nadciśnienie z brakiem spadku nocnego. Badania laboratoryjne w normie, potas cztery i dwa, kreatynina zero osiemdziesiąt pięć. Rozpoznaję nadciśnienie tętnicze trzeciego stopnia z przerostem lewej komory. Włączam indapamid dwa i pół miligrama rano i zalecam zmianę ramiprilu na telmisartan osiemdziesiąt miligramów ze względu na lepszą tolerancję dobową. Odstawienie palenia bezwzględne, konsultacja w poradni antynikotynowej. Kontrola u mnie za trzy miesiące z aktualnym holterem ciśnieniowym. Proszę o dalszą opiekę POZ między wizytami.";

    let example_output = "PORADNIA KARDIOLOGICZNA
[Miejscowość], [data]

Sz. P. dr n. med. Nowak
Praktyka Lekarza Rodzinnego

Dot. pacjenta: [imię i nazwisko], PESEL: [___]

Szanowna Pani Doktor,

Dziękuję za skierowanie pacjenta na konsultację kardiologiczną.

58-letni pacjent skierowany z powodu nadciśnienia tętniczego opornego na leczenie dwulekowe (amlodypina 5 mg + ramipril 10 mg). W domowych pomiarach średnie ciśnienie 155/95 mmHg. Palacz (ok. 20 paczkolat), BMI 31 kg/m².

W badaniu przedmiotowym bez cech niewydolności krążenia.
ECHO: przerost lewej komory, EF 60%, zastawki bez istotnych wad.
Holter RR: potwierdzone nadciśnienie z brakiem spadku nocnego (non-dipper).
Laboratoryjnie: K 4,2 mmol/l, kreatynina 0,85 mg/dl – w normie.

ROZPOZNANIE
Nadciśnienie tętnicze 3. stopnia z przerostem lewej komory.

PROPONOWANE POSTĘPOWANIE
1. Zmiana ramiprilu 10 mg na telmisartan 80 mg 1×/d (lepsze pokrycie dobowe u non-dippera).
2. Dołączenie indapamidu 2,5 mg 1×/d rano.
3. Bezwzględne zaprzestanie palenia – sugeruję skierowanie do poradni antynikotynowej.
4. Kontynuacja amlodypiny 5 mg 1×/d.

KONTROLA
W Poradni Kardiologicznej za 3 miesiące, z aktualnym 24-godzinnym pomiarem ciśnienia.

Uprzejmie proszę o dalszą opiekę POZ oraz monitorowanie tolerancji terapii między wizytami specjalistycznymi.

Z wyrazami szacunku,
dr n. med. [imię i nazwisko]
specjalista kardiolog";

    builtin(
        "builtin-consultation",
        "List konsultacyjny",
        "List specjalisty do lekarza kierującego (POZ / inny specjalista)",
        content,
        example_input,
        example_output,
    )
}

fn surgery_protocol() -> Template {
    let content = "Przekształć dyktowany opis zabiegu w oficjalny protokół operacyjny zgodny z polskimi standardami dokumentacji chirurgicznej. Rozpoznania przed- i pooperacyjne podawaj z kodem ICD-10, nazwę procedury z kodem ICD-9. PRZEBIEG ZABIEGU formatuj narracyjnie (nie bullet points), z wyraźnym podziałem na etapy. ZNALEZISKA ŚRÓDOPERACYJNE i ZALECENIA POOPERACYJNE — punktowo. Jeśli lekarz nie wspomniał o powikłaniach, materiale, stracie krwi — wpisz wyraźnie „brak\" / „nie pobierano\" / „minimalna\".

Struktura wyjściowa:

[Dane administracyjne — auto]
  Data, czas rozpoczęcia/zakończenia, zespół operacyjny, znieczulenie

ROZPOZNANIE PRZEDOPERACYJNE [ICD-10]
ROZPOZNANIE POOPERACYJNE [ICD-10]
NAZWA PROCEDURY [ICD-9]

ZNIECZULENIE

PRZEBIEG ZABIEGU (narracyjnie)
  - Ułożenie pacjenta, dostępy
  - Kolejne kroki zabiegu
  - Znaleziska śródoperacyjne
  - Zamknięcie

ZNALEZISKA ŚRÓDOPERACYJNE (punktowo)
MATERIAŁ DO BADAŃ (hist-pat, posiew)
POWIKŁANIA
STRATA KRWI
STAN PO ZABIEGU
ZALECENIA POOPERACYJNE (numerowane)";

    let example_input = "Artroskopia kolana prawego u pacjenta trzydziestodwuletniego z podejrzeniem uszkodzenia łąkotki przyśrodkowej po urazie skrętnym. Znieczulenie podpajęczynówkowe, opaska uciskowa na udzie. Ułożenie na plecach z kolanem zgiętym pod kątem dziewięćdziesiąt stopni. Dostęp standardowy przednio-boczny i przednio-przyśrodkowy. Po wprowadzeniu artroskopu wizualizacja zachyłka nadrzepkowego - bez zmian. Staw rzepkowo-udowy, chrząstka rzepki drugi stopień uszkodzenia według Outerbridge'a, tylko niewielkie rozwłóknienia. Przedział przyśrodkowy - rozdarcie rogu tylnego łąkotki przyśrodkowej typu koszykowego, łąkotka niestabilna. Wykonałem częściową meniscektomię przyśrodkową, usunięto fragment oderwany, brzeg wygładzono. Przedział boczny łąkotka zachowana. Więzadła krzyżowe ciągłe, stabilne test Lachmana. Płukanie stawu, usunięcie narzędzi, szwy pojedyncze na skórę, opatrunek jałowy z kompresją. Strata krwi minimalna. Bez powikłań. Pacjent wybudzony, stan dobry. Po zabiegu chłodzenie, elewacja, obciążanie z pełnym podparciem od razu, rehabilitacja od drugiej doby, kontrola za dziesięć dni, usunięcie szwów.";

    let example_output = "PROTOKÓŁ ZABIEGU OPERACYJNEGO

ROZPOZNANIE PRZEDOPERACYJNE
Podejrzenie uszkodzenia łąkotki przyśrodkowej kolana prawego po urazie skrętnym. [ICD-10: S83.2]

ROZPOZNANIE POOPERACYJNE
Rozdarcie typu koszykowego rogu tylnego łąkotki przyśrodkowej kolana prawego. Chondromalacja rzepki II° (Outerbridge).

NAZWA PROCEDURY
Artroskopia kolana prawego z częściową meniscektomią przyśrodkową [ICD-9: 80.26]

ZNIECZULENIE
Podpajęczynówkowe. Opaska uciskowa na udzie.

PRZEBIEG ZABIEGU
Pacjent w ułożeniu na plecach, kolano prawe zgięte pod kątem 90°. Po przygotowaniu pola operacyjnego wykonano standardowe dostępy przednio-boczny i przednio-przyśrodkowy.

Po wprowadzeniu artroskopu uwidoczniono zachyłek nadrzepkowy bez zmian patologicznych. W stawie rzepkowo-udowym stwierdzono chondromalację rzepki II° wg Outerbridge'a (niewielkie rozwłóknienia chrząstki). W przedziale przyśrodkowym uwidoczniono rozdarcie rogu tylnego łąkotki przyśrodkowej typu koszykowego z niestabilną łąkotką. Wykonano częściową meniscektomię przyśrodkową – usunięto fragment oderwany, brzeg pozostałej części łąkotki wygładzono. Przedział boczny oraz więzadła krzyżowe (test Lachmana ujemny śródoperacyjnie) bez zmian.

Wykonano obfite płukanie stawu. Po usunięciu narzędzi rany zamknięto szwami pojedynczymi na skórę. Opatrunek jałowy z kompresją.

ZNALEZISKA ŚRÓDOPERACYJNE
- Chondromalacja rzepki II° wg Outerbridge'a
- Rozdarcie rogu tylnego łąkotki przyśrodkowej typu koszykowego
- Łąkotka boczna oraz więzadła krzyżowe bez zmian

MATERIAŁ DO BADAŃ
Nie pobierano.

POWIKŁANIA
Brak.

STRATA KRWI
Minimalna.

STAN PO ZABIEGU
Pacjent wybudzony, stan ogólny dobry, parametry stabilne.

ZALECENIA POOPERACYJNE
1. Chłodzenie kolana (3×/d po 20 min) przez 48 h, elewacja kończyny.
2. Pełne obciążanie kończyny operowanej od razu, z asekuracją kul przez 2-3 dni.
3. Rehabilitacja: izometria mięśnia czworogłowego od 2. doby; skierowanie do fizjoterapeuty.
4. Kontrola w Poradni Ortopedycznej za 10 dni – usunięcie szwów.
5. Profilaktyka przeciwzakrzepowa – nie wymagana.";

    builtin(
        "builtin-surgery",
        "Protokół zabiegu",
        "Protokół operacyjny: rozpoznania, przebieg, znaleziska, zalecenia",
        content,
        example_input,
        example_output,
    )
}
