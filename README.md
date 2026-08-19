**_Minden jog fenntartva. Készítő: Benya_**

# Minecraft-módosítócsomag-frissítő (MMF)

Az MMF alkalmazás lehetővé teszi, hogy egy _mods_ mappán belüli _.jar_ fájlok egy távoli tárolóhoz "igazodjanak". (A program elsősorban Windows operációs rendszerre készült.)

Egy privát emcészerver tulajdonosa feltöltheti a mindenkori szükséges modokat a távoli tárolóba, hogy a szerver valamennyi tagja letölthesse azokat az MMF-fel. (Ebből kifolyólag az alkalmazás működéséhez internetkapcsolatra van szükségünk.)

Az MMF nemcsak az "új" modokat tölti le, hanem a "régieket" is eltávolítja.

Van lehetőség egy módosítócsomag MMF általi kezelésének letiltására is. Ekkor a program az adott modot sem letölteni nem fogja, sem eltávolítani.

## Az MMF használata

Mindenekelőtt érdemes lehet olyan nevet adni az _.exe_-fájlnak, mellyel az a fájlkezelőben a mappa "legtetejére" kerül. Pl. "**_!_**_mmf.exe_".
Helyezzük a fájlt a _mods_ mappánkba.

Az alkalmazás első megnyitásakor létrejön egy _!mmf.yaml_ konfigurációs fájl. Ebben állítható be a sötét téma is: **_dark-theme: true_**

A legfontosabb azonban a távoli tároló elérhetősége, amit az egyszerűség kedvéért nevezzünk szolgáltatónak.

A szolgáltató címe felől minden esetben a szervertulajdonosnál érdeklődjünk, ez ugyanis az ő hatásköre.

A szolgáltató címének megadása után (pl. _example.com/modpack_) az alkalmazást megnyitva bármikor frissíthetjük módosítócsomagjainkat. Ha egy modot nem szeretnénk letölteni vagy eltávolítani, akkor vegyük ki a neve elől a pipát.

A módosítások csak a _Módosítócsomag-frissítés_ gombra való kattintással lépnek életbe.

## Tárolók létrehozása (szervertulajdonosoknak)

A távoli tároló tulajdonképpen egy HTTP-szerver, vagyis egy módosítócsomag-szolgáltató létrehozásához elengendő egy honlappal rendelkeznünk. (Egy statikus is megfelelő.)

Hozzunk létre honlapunkon egy módosítócsomag-gyökérmappát _(mócsgyöm)_.

A mócsgyömön belül egy _mods_ nevű mappában helyezzük el valamennyi módosítócsomagunkat.

Szintén a mócsgyömön belül hozzunk létre egy _mods.json_ nevű fájlt, melyben egyetlen JavaScript-tömb meglétéről gondoskodjunk. E tömb elemei legyenek a mócsgyömi _mods_ mappában levő módosítócsomagok fájlnevei.

A _mods.json_ fájl létrehozását akár egy Python-szkripttel is önműködősíthetjük:

```py
from json import dump
from os import listdir

PATH = "[...]/modpack"  # a mócsgyöm elérési útja

with open(PATH + "/mods.json", "w", encoding="utf8") as f:
    dump(listdir(PATH + "/mods"), f)
```

Ha honlapunk doménneve _example.com_, és a mócsgyöm a _/modpack_ mappa; akkor a "szolgáltatónk" az _example.com/modpack_ lesz. Ezt tudassuk szerverünk valamennyi ügyfelével.

## Az MMF működése

Az alkalmazás a megnyitását követően feljegyzi a helyi és a távoli _mods_ mappában található módosítócsomagok neveit. Ezen adatok alapján számítja ki, hogy melyek a letöltendő, illetve eltávolítandó modok.

Az MMF nemcsak azért nagyon gyors, mert Rust nyelven íródott; hanem azért is, mert a sebesség maximalizálása érdekében mind a letöltéseket, mind pedig az eltávolításokat párhuzamosan végzi. A program képes kihasználni a rendelkezésre álló teljes sávszélességet.

Sikerekben gazdag módosítócsomag-frissítést kívánok!
