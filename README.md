<div align="center">
<img src="rnote-ui/data/icons/scalable/apps/rnote.svg" width="300"></img>
</div><br><br><br>

<div align="center">
    <a href="https://github.com/flxzt/rnote/actions/workflows/ci.yml">
        <img alt="CI"src="https://github.com/flxzt/rnote/actions/workflows/ci.yml/badge.svg"></img>
    </a>
    <a href="https://liberapay.com/flxzt/donate">
        <img alt="Donate using Liberapay" src="https://github.com/flxzt/rnote/blob/main/misc/assets/liberapay-donate-button.svg" width="60" height="20">
    </a>
</div>


# Rnote

> Sketch and take handwritten notes.  

Rnote is an open-source vector-based drawing app for sketching, handwritten notes and to annotate documents and pictures. Targeted at students, teachers and those who own a drawing tablet, it provides features like PDF and picture import and export, an infinite canvas and an adaptive UI for big and small screens.

Written in Rust and GTK4.

**Features**

- Adaptive UI focused on stylus input
- Pressure-sensitive stylus input with different and configurable stroke styles
- Create many different shapes with the shape tool
- Move, rotate, resize and modify existing content with the selection tool
- Different document expansion layouts ( fixed pages, continuous vertical, infinite in every direction, .. )
- Customizable background colors, patterns, sizes
- Customizable page format
- (Optional) pen sounds
- Reconfigurable stylus button shortcuts
- An integrated workspace browser for quick access to related files
- Drag & Drop, clipboard support
- PDF, Bitmap and SVG image import
- Document, document pages and selection export to many formats including SVG, PDF, Xopp
- Save and load the documents in the native `.rnote` file format
- Tabs to work on multiple documents at the same time
- Autosave, printing

**Disclaimer**

The file format is still unstable. It might change and break compatibility between versions.

## Website

Rnote has a project website: [rnote.flxzt.net](https://rnote.flxzt.net/)

## Installation

Rnote is available as a flatpak on Flathub:

<br><div align="start">
<a href='https://flathub.org/apps/details/com.github.flxzt.rnote'><img width="256" alt='Download on Flathub' src='https://flathub.org/assets/badges/flathub-badge-en.png'/></a>
</div><br>

**Downgrading**

Because the file format still is unstable, downgrading to a specific version might be necessary.

List all available past versions on flathub:

```bash
flatpak remote-info --log flathub com.github.flxzt.rnote
```

Pick the commit from the desired version and downgrade with:

```bash
sudo flatpak update --commit=<commit-hash> com.github.flxzt.rnote 
```

After downgrading, the flatpak can be pinned or unpinned with:

```
$ flatpak mask com.github.flxzt.rnote
$ flatpak mask --remove com.github.flxzt.rnote
```

To update to the lastest version again, unpin and run `flatpak update`.

## Screenshots

![overview](./rnote-ui/data/screenshots/overview.png)  
![lecture_note_1](./rnote-ui/data/screenshots/lecture_note_1.png)  
![pdf_annotation](./rnote-ui/data/screenshots/pdf_annotation.png)  
![lecture_note_2](./rnote-ui/data/screenshots/lecture_note_2.png)  

## Pitfalls & Known Issues

* Drag & Drop not working -  
    Make sure Rnote has permissions to the locations you are dragging files from. Can be granted in Flatseal (a Flatpak permissions manager)

* Odd location for current file -  
    When the directory displayed in the header title is something like `/run/user/1000/../`, rnote does not have permissions to access the directory. Again, granting them in Flatseal fixes this issue.

* Stylus buttons move canvas / are not functional -  
    Make sure that the `xf86-input-wacom`, drivers on X11 and `libinput` on Wayland and `libwacom` are installed and loaded.

* While hovering with the stylus, other input events are blocked in some regions of the screen -  
    Supposed to be palm rejection, but might be undesirable. If there is a left- / righthanded system tablet setting, make sure it is set correctly. Rnote can't disable this unfortunately. ( discussed in issue [#329](https://github.com/flxzt/rnote/issues/329) )

* One of the stylus buttons shortcut mapping does not work as intended -  
On some devices one stylus button is mapped to a dedicated "Eraser" mode (which is the back-side on other styli). The buttons in the shortcuts settings could then be inconsistent ( the secondary / upper button is actually the primary / lower button , or reverse ). To change the tool that is mapped to this "Eraser" mode, do the following:  
    * Hover over the canvas, and press and hold the button that that is suspected to be mapped to the "Eraser" mode
    * Switch to the desired pen style while keeping the button pressed
    * When releasing the pressed button, it should switch back to the previous pen style
    * The pen style in the "Eraser" mode should now be remembered

## Credits

- A huge thanks to the contributors, translators and to all that donated. You are the ones that help keep the project going!
- [Freesound](https://freesound.org/) is the source for the pen sounds. The individual sounds are credited in `sounds/Licenses.md`
- [Rough.js](https://roughjs.com/) provides the algorithms for implementation of Rnote's rough shapes.
- [Pizarra](https://pizarra.categulario.xyz/en/) is an innovative drawing app with advanced shaping and featuring an infinite zoom. It is a great inspiration of the architecture of Rnote. Go check it out!

## Translations

<a href="https://hosted.weblate.org/engage/rnote/">
<img src="https://hosted.weblate.org/widgets/rnote/-/repo/multi-auto.svg" alt="Translation status" />
</a><br><br>

A great way to contribute to the project without writing code is adding a new or start maintaining an existing translation language. The translations files are located in `rnote-ui/po/`.

 Creating translations for new languages or updating existing ones can be done in two ways:
- take the `rnote.pot` file and generate a new `.po` translation file from it, for example with "Poedit". Add the new translation language to `LINGUAS` and submit a PR with both changed files.
- use [weblate](https://hosted.weblate.org/projects/rnote/repo/) for an easy way to translate in the browser without having to deal with git.

## Community

If you have any questions or want to start a general discussion, open a topic in the [Github Discussions](https://github.com/flxzt/rnote/discussions) section.  
There is also the [#rnote:matrix.org](https://matrix.to/#/#rnote:matrix.org) chat room.  

## File Format

The `.rnote` file format is a gzipped json file. It is (de)compressed with the `flate2` crate and (de)serialized with the `Serde` crate.

So far breaking changes in the format happened in versions:

- `v0.2.0`
- `v0.3.0`
- `v0.4.0`
- `v0.5.0`

To be able to open and export older files that are incompatible with the newest version, look under **Installation** /**Downgrading** to install older versions of Rnote.

## Drawings Created With Rnote

If you have drawn something cool in Rnote and want to share it, submit a PR so it can be showcased here. :)  

<div align="center" spacing="20px">
        <img alt="Pikachu" src="https://github.com/flxzt/rnote/blob/main/misc/drawings/pikachu.png" height="400">
        <img alt="Tree" src="https://github.com/flxzt/rnote/blob/main/misc/drawings/tree.svg" height="400">
        <img alt="Love" src="https://github.com/flxzt/rnote/blob/main/misc/drawings/love.png" height="400">
</div><br>


## Building

Build instructions for Linux are documented in [BUILDING.md](./BUILDING.md) and for other platforms [here](./misc/building)
