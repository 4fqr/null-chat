#!/usr/bin/env python3
"""
NullChat – Python GUI (CustomTkinter)
Communicates with the Rust backend via JSON-lines over TCP.
Usage: python3 gui/main.py <PORT>
"""

import sys
import os
import socket
import threading
import json
import time
import tkinter as tk
from tkinter import messagebox
import customtkinter as ctk
from datetime import datetime

# ─── Theme constants ──────────────────────────────────────────────────────────
BG_DARKEST  = "#1e2124"
BG_DARK     = "#2c2f33"
BG_MAIN     = "#36393f"
BG_CARD     = "#2f3136"
BG_HOVER    = "#3c3f44"
BG_SELECTED = "#404449"
BG_INPUT    = "#40444b"
TEXT        = "#dcddde"
TEXT_MUTED  = "#72767d"
TEXT_BRIGHT = "#ffffff"
BLURPLE     = "#5865f2"
BLURPLE_D   = "#4752c4"
GREEN       = "#57f287"
RED         = "#ed4245"
YELLOW      = "#fee75c"
TEAL        = "#00b0f4"
DIVIDER     = "#292b2f"

FONT        = ("Segoe UI", 13)
FONT_SM     = ("Segoe UI", 11)
FONT_XS     = ("Segoe UI", 10)
FONT_LG     = ("Segoe UI", 16, "bold")
FONT_TITLE  = ("Segoe UI", 20, "bold")
MONO        = ("Consolas", 11)

# ─── IPC ──────────────────────────────────────────────────────────────────────

class IPC:
    def __init__(self, port: int):
        self.port   = port
        self._sock  = None
        self._fobj  = None
        self._lock  = threading.Lock()
        self._alive = False

    def connect(self, timeout=30) -> bool:
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                s.settimeout(2)
                s.connect(("127.0.0.1", self.port))
                s.settimeout(None)
                self._sock  = s
                self._fobj  = s.makefile("r", encoding="utf-8")
                self._alive = True
                return True
            except (ConnectionRefusedError, OSError):
                time.sleep(0.5)
        return False

    def send(self, obj: dict):
        with self._lock:
            if not self._sock:
                return
            try:
                line = json.dumps(obj) + "\n"
                self._sock.sendall(line.encode())
            except OSError:
                self._alive = False

    def readline(self):
        """Blocking; returns dict or None on error."""
        try:
            line = self._fobj.readline()
            if not line:
                self._alive = False
                return None
            return json.loads(line.strip())
        except (OSError, json.JSONDecodeError, ValueError):
            self._alive = False
            return None

    def close(self):
        self._alive = False
        try:
            if self._sock:
                self._sock.close()
        except OSError:
            pass

# ─── Helpers ─────────────────────────────────────────────────────────────────

def ts_to_str(ts: int) -> str:
    try:
        dt = datetime.fromtimestamp(ts)
        return dt.strftime("Today %H:%M") if dt.date() == datetime.today().date() else dt.strftime("%d/%m/%Y %H:%M")
    except Exception:
        return ""

def make_avatar(parent, text: str, size=32, bg=BLURPLE, fg=TEXT_BRIGHT) -> ctk.CTkLabel:
    initials = "".join(w[0].upper() for w in text.split()[:2]) if text else "?"
    lbl = ctk.CTkLabel(parent, text=initials[:2], width=size, height=size,
                        fg_color=bg, corner_radius=size//2,
                        text_color=fg, font=("Segoe UI", size//3 + 2, "bold"))
    return lbl

def status_color(s: str) -> str:
    return {"online": GREEN, "away": YELLOW, "dnd": RED, "offline": TEXT_MUTED}.get(s, TEXT_MUTED)

# ─── Scrollable frame helper ──────────────────────────────────────────────────

class ScrollFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, **kw):
        super().__init__(master, fg_color="transparent", **kw)
        self.grid_columnconfigure(0, weight=1)

# ─── Modals ───────────────────────────────────────────────────────────────────

class ModalBase(ctk.CTkToplevel):
    def __init__(self, parent, title: str, width=420, height=350):
        super().__init__(parent)
        self.title(title)
        self.geometry(f"{width}x{height}")
        self.resizable(False, False)
        self.configure(fg_color=BG_DARK)
        self.grab_set()
        self.focus_set()
        # Centre on parent
        px = parent.winfo_rootx() + parent.winfo_width()  // 2 - width  // 2
        py = parent.winfo_rooty() + parent.winfo_height() // 2 - height // 2
        self.geometry(f"+{px}+{py}")

        ctk.CTkLabel(self, text=title, font=FONT_LG, text_color=TEXT_BRIGHT).pack(pady=(18,8))


class AddFriendModal(ModalBase):
    def __init__(self, parent, on_submit):
        super().__init__(parent, "Add Friend", 380, 240)
        self._cb = on_submit
        ctk.CTkLabel(self, text="User ID (fingerprint)", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._uid = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT, width=330)
        self._uid.pack(padx=24, pady=(4,8), fill="x")
        ctk.CTkLabel(self, text="Display Name", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._name = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT, width=330)
        self._name.pack(padx=24, pady=(4,14), fill="x")
        ctk.CTkButton(self, text="Add", fg_color=BLURPLE, hover_color=BLURPLE_D, command=self._ok).pack()

    def _ok(self):
        uid = self._uid.get().strip(); name = self._name.get().strip()
        if uid:
            self._cb(uid, name or uid[:8])
            self.destroy()


class CreateGroupModal(ModalBase):
    def __init__(self, parent, on_submit):
        super().__init__(parent, "Create Group", 380, 220)
        self._cb = on_submit
        ctk.CTkLabel(self, text="Group Name", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._name = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT)
        self._name.pack(padx=24, pady=(4,8), fill="x")
        ctk.CTkLabel(self, text="Description (optional)", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._desc = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT)
        self._desc.pack(padx=24, pady=(4,14), fill="x")
        ctk.CTkButton(self, text="Create", fg_color=BLURPLE, hover_color=BLURPLE_D, command=self._ok).pack()

    def _ok(self):
        name = self._name.get().strip()
        if name:
            self._cb(name, self._desc.get().strip())
            self.destroy()


class JoinServerModal(ModalBase):
    def __init__(self, parent, on_submit):
        super().__init__(parent, "Join Server", 380, 190)
        self._cb = on_submit
        ctk.CTkLabel(self, text="Invite Code", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._code = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT)
        self._code.pack(padx=24, pady=(4,14), fill="x")
        ctk.CTkButton(self, text="Join", fg_color=BLURPLE, hover_color=BLURPLE_D, command=self._ok).pack()

    def _ok(self):
        code = self._code.get().strip()
        if code:
            self._cb(code)
            self.destroy()


class EditProfileModal(ModalBase):
    def __init__(self, parent, state: dict, on_submit):
        super().__init__(parent, "Edit Profile", 400, 320)
        self._cb = on_submit
        ctk.CTkLabel(self, text="Display Name", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._name = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT)
        self._name.insert(0, state.get("my_name",""))
        self._name.pack(padx=24, pady=(4,8), fill="x")
        ctk.CTkLabel(self, text="Nickname (optional)", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._nick = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT)
        self._nick.insert(0, state.get("my_nick") or "")
        self._nick.pack(padx=24, pady=(4,8), fill="x")
        ctk.CTkLabel(self, text="Bio (optional)", font=FONT_SM, text_color=TEXT_MUTED).pack(padx=24, anchor="w")
        self._bio = ctk.CTkEntry(self, fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT)
        self._bio.insert(0, state.get("my_bio") or "")
        self._bio.pack(padx=24, pady=(4,14), fill="x")
        row = ctk.CTkFrame(self, fg_color="transparent")
        row.pack()
        ctk.CTkLabel(row, text="Status:", text_color=TEXT_MUTED, font=FONT_SM).pack(side="left", padx=(0,8))
        self._status = ctk.CTkOptionMenu(row, values=["online","away","dnd","offline"],
                                          fg_color=BG_INPUT, button_color=BLURPLE)
        self._status.set(state.get("my_status","online"))
        self._status.pack(side="left", pady=(0,14))
        ctk.CTkButton(self, text="Save", fg_color=BLURPLE, hover_color=BLURPLE_D, command=self._ok).pack(pady=10)

    def _ok(self):
        self._cb(self._name.get().strip(), self._nick.get().strip(), self._bio.get().strip(), self._status.get())
        self.destroy()


class MembersModal(ModalBase):
    def __init__(self, parent, context_name, members: list, my_id: str, is_owner: bool, on_action):
        super().__init__(parent, f"Members – {context_name}", 420, 480)
        self._cb = on_action
        ctk.CTkLabel(self, text=f"{len(members)} members", font=FONT_SM, text_color=TEXT_MUTED).pack()
        sf = ScrollFrame(self, width=380, height=340)
        sf.pack(padx=10, pady=8, fill="both", expand=True)
        for m in members:
            row = ctk.CTkFrame(sf, fg_color=BG_CARD, corner_radius=6)
            row.grid(sticky="ew", pady=2, padx=4)
            row.grid_columnconfigure(1, weight=1)
            make_avatar(row, m.get("display_name","?"), size=28).grid(row=0,col=0, padx=(8,6),pady=6)
            info = ctk.CTkFrame(row, fg_color="transparent")
            info.grid(row=0,column=1,sticky="w")
            role_color = {"owner": YELLOW, "admin": RED, "moderator": BLURPLE}.get(m.get("role",""),"")
            ctk.CTkLabel(info, text=m.get("display_name","?"), font=FONT, text_color=TEXT).pack(anchor="w")
            ctk.CTkLabel(info, text=m.get("role","member").capitalize(), font=FONT_XS,
                          text_color=role_color or TEXT_MUTED).pack(anchor="w")
            if is_owner and m.get("user_id") != my_id:
                btn = ctk.CTkButton(row, text="⋮", width=28, fg_color="transparent",
                                     text_color=TEXT_MUTED, hover_color=BG_HOVER)
                uid = m.get("user_id","")
                btn.configure(command=lambda uid=uid: on_action(uid, m.get("display_name","")))
                btn.grid(row=0,column=2,padx=6)


class ContextMenu(tk.Menu):
    def __init__(self, parent):
        super().__init__(parent, tearoff=False, bg=BG_DARK, fg=TEXT,
                         activebackground=BG_HOVER, activeforeground=TEXT_BRIGHT,
                         borderwidth=1, relief="flat", font=FONT_SM)

    def popup(self, event):
        try:
            self.tk_popup(event.x_root, event.y_root)
        finally:
            self.grab_release()

# ─── Setup / Unlock screen ────────────────────────────────────────────────────

class SetupScreen(ctk.CTkFrame):
    def __init__(self, parent, ipc: IPC, is_first: bool, on_done):
        super().__init__(parent, fg_color=BG_DARKEST)
        self._ipc     = ipc
        self._cb      = on_done
        self._first   = is_first
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(0, weight=1)

        card = ctk.CTkFrame(self, fg_color=BG_DARK, corner_radius=16, width=380)
        card.place(relx=0.5, rely=0.5, anchor="center")
        card.grid_propagate(False)

        # Logo area
        logo = ctk.CTkLabel(card, text="⬡ NullChat", font=("Segoe UI", 26, "bold"),
                             text_color=BLURPLE)
        logo.pack(pady=(28,4))
        ctk.CTkLabel(card, text="Encrypted · Anonymous · Sovereign",
                     font=FONT_XS, text_color=TEXT_MUTED).pack(pady=(0,20))

        if is_first:
            ctk.CTkLabel(card, text="Choose your name", font=FONT_SM, text_color=TEXT_MUTED).pack(anchor="w", padx=32)
            self._name = ctk.CTkEntry(card, placeholder_text="Anonymous", fg_color=BG_INPUT,
                                       border_color=DIVIDER, text_color=TEXT, width=316)
            self._name.pack(padx=32, pady=(4,12))

        ctk.CTkLabel(card, text="Vault passphrase", font=FONT_SM, text_color=TEXT_MUTED).pack(anchor="w", padx=32)
        self._pass = ctk.CTkEntry(card, show="•", placeholder_text="••••••••",
                                   fg_color=BG_INPUT, border_color=DIVIDER, text_color=TEXT, width=316)
        self._pass.pack(padx=32, pady=(4,8))
        self._pass.bind("<Return>", lambda _: self._submit())

        if is_first:
            ctk.CTkLabel(card, text="Confirm passphrase", font=FONT_SM, text_color=TEXT_MUTED).pack(anchor="w", padx=32)
            self._pass2 = ctk.CTkEntry(card, show="•", fg_color=BG_INPUT,
                                        border_color=DIVIDER, text_color=TEXT, width=316)
            self._pass2.pack(padx=32, pady=(4,8))
            self._pass2.bind("<Return>", lambda _: self._submit())

        self._err = ctk.CTkLabel(card, text="", font=FONT_SM, text_color=RED)
        self._err.pack(pady=2)

        btn_label = "Create Account" if is_first else "Unlock"
        ctk.CTkButton(card, text=btn_label, fg_color=BLURPLE, hover_color=BLURPLE_D,
                       command=self._submit, height=38, corner_radius=8,
                       font=("Segoe UI",13,"bold")).pack(padx=32, pady=(4,24))

    def show_error(self, msg: str):
        self._err.configure(text=msg)

    def _submit(self):
        pw = self._pass.get().strip()
        if not pw:
            self.show_error("Passphrase required.")
            return
        if self._first:
            pw2 = self._pass2.get().strip()
            if pw != pw2:
                self.show_error("Passphrases do not match.")
                return
            name = self._name.get().strip() or "Anonymous"
            self._ipc.send({"cmd": "setup", "name": name, "pass": pw})
        else:
            self._ipc.send({"cmd": "unlock", "pass": pw})
        self._cb()

# ─── Loading overlay ──────────────────────────────────────────────────────────

class LoadingScreen(ctk.CTkFrame):
    def __init__(self, parent):
        super().__init__(parent, fg_color=BG_DARKEST)
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(0, weight=1)
        c = ctk.CTkFrame(self, fg_color="transparent")
        c.place(relx=0.5, rely=0.5, anchor="center")
        ctk.CTkLabel(c, text="⬡", font=("Segoe UI", 60), text_color=BLURPLE).pack()
        ctk.CTkLabel(c, text="NullChat", font=("Segoe UI", 24, "bold"), text_color=TEXT_BRIGHT).pack()
        self._lbl = ctk.CTkLabel(c, text="Connecting to backend…", font=FONT_SM, text_color=TEXT_MUTED)
        self._lbl.pack(pady=10)
        self._bar = ctk.CTkProgressBar(c, width=260, mode="indeterminate", progress_color=BLURPLE)
        self._bar.pack(pady=6)
        self._bar.start()

    def set_status(self, msg: str):
        self._lbl.configure(text=msg)

# ─── Main chat view ───────────────────────────────────────────────────────────

class MessageItem(ctk.CTkFrame):
    def __init__(self, parent, msg: dict, **kw):
        super().__init__(parent, fg_color="transparent", **kw)
        self.grid_columnconfigure(1, weight=1)
        avatar_bg = BLURPLE if msg.get("outgoing") else "#4a4d52"
        av = make_avatar(self, msg.get("from_name","?"), size=36, bg=avatar_bg)
        av.grid(row=0, column=0, padx=(0,10), pady=(6,0), sticky="n")
        content = ctk.CTkFrame(self, fg_color="transparent")
        content.grid(row=0, column=1, sticky="ew")
        content.grid_columnconfigure(0, weight=1)
        header = ctk.CTkFrame(content, fg_color="transparent")
        header.grid(sticky="ew")
        name_color = BLURPLE if msg.get("outgoing") else TEXT_BRIGHT
        ctk.CTkLabel(header, text=msg.get("from_name","?"), font=("Segoe UI",13,"bold"),
                      text_color=name_color).pack(side="left")
        ctk.CTkLabel(header, text=f"  {ts_to_str(msg.get('ts',0))}", font=FONT_XS,
                      text_color=TEXT_MUTED).pack(side="left", pady=(2,0))
        body_txt = msg.get("body","")
        body = ctk.CTkLabel(content, text=body_txt, font=FONT, text_color=TEXT,
                             wraplength=540, justify="left", anchor="w")
        body.grid(sticky="ew", pady=(2,6))


class ChatArea(ctk.CTkFrame):
    def __init__(self, parent, ipc: IPC):
        super().__init__(parent, fg_color=BG_MAIN)
        self._ipc      = ipc
        self._ctx      = None   # dict with 'type','id','name','ch_type','server_id'
        self._state    = {}

        self.grid_rowconfigure(1, weight=1)
        self.grid_columnconfigure(0, weight=1)
        self.grid(sticky="nsew")

        # Header
        self._header = ctk.CTkFrame(self, fg_color=BG_DARK, height=52, corner_radius=0)
        self._header.grid(row=0, column=0, sticky="ew")
        self._header.grid_columnconfigure(1, weight=1)
        self._header.grid_propagate(False)
        self._hicon = ctk.CTkLabel(self._header, text="#", font=("Segoe UI",18,"bold"),
                                    text_color=TEXT_MUTED, width=30)
        self._hicon.grid(row=0, column=0, padx=(16,4), pady=14)
        self._hname = ctk.CTkLabel(self._header, text="Select a conversation",
                                    font=("Segoe UI",14,"bold"), text_color=TEXT_BRIGHT)
        self._hname.grid(row=0, column=1, sticky="w", pady=14)
        self._htopic = ctk.CTkLabel(self._header, text="", font=FONT_XS, text_color=TEXT_MUTED)
        self._htopic.grid(row=0, column=2, padx=16, pady=14)

        # Messages
        self._msg_frame = ScrollFrame(self, width=0)
        self._msg_frame.grid(row=1, column=0, sticky="nsew", padx=0, pady=0)

        # Empty state
        self._empty = ctk.CTkLabel(self._msg_frame, text="No messages yet.\nBe the first to say something.",
                                    font=FONT, text_color=TEXT_MUTED)
        self._empty.grid(row=0, column=0, pady=40)

        # Compose bar
        self._compose = ctk.CTkFrame(self, fg_color=BG_DARK, height=64, corner_radius=0)
        self._compose.grid(row=2, column=0, sticky="ew")
        self._compose.grid_columnconfigure(0, weight=1)
        self._compose.grid_propagate(False)
        self._input = ctk.CTkEntry(self._compose, placeholder_text="Message…",
                                    fg_color=BG_INPUT, border_color=BG_INPUT,
                                    text_color=TEXT, height=40, corner_radius=8)
        self._input.grid(row=0, column=0, padx=(12,8), pady=12, sticky="ew")
        self._input.bind("<Return>", lambda _: self._send())
        self._send_btn = ctk.CTkButton(self._compose, text="➤", width=40, height=40,
                                        fg_color=BLURPLE, hover_color=BLURPLE_D,
                                        corner_radius=8, command=self._send)
        self._send_btn.grid(row=0, column=1, padx=(0,12), pady=12)

        self._owner_bar = ctk.CTkLabel(self._compose,
                                        text="🔒 Owner-only channel — read only",
                                        font=FONT_SM, text_color=TEXT_MUTED)

    def set_context(self, ctx: dict, state: dict):
        self._ctx   = ctx
        self._state = state
        self._render_header()
        self._render_messages()

    def _render_header(self):
        if not self._ctx:
            return
        t = self._ctx.get("type","")
        name = self._ctx.get("name","")
        icons = {"channel":"#", "dm":"@", "group":"⊕"}
        self._hicon.configure(text=icons.get(t,"#"))
        self._hname.configure(text=name)
        topic = self._ctx.get("topic") or ""
        self._htopic.configure(text=topic[:60] if topic else "")
        # restricted?
        restricted = self._ctx.get("restricted", False)
        self._owner_bar.grid_forget()
        self._input.grid_forget()
        self._send_btn.grid_forget()
        if restricted:
            self._owner_bar.grid(row=0, column=0, columnspan=2, padx=20, pady=20)
            self._input.configure(state="disabled")
        else:
            self._input.grid(row=0, column=0, padx=(12,8), pady=12, sticky="ew")
            self._send_btn.grid(row=0, column=1, padx=(0,12), pady=12)
            self._input.configure(state="normal")

    def _render_messages(self):
        for w in self._msg_frame.winfo_children():
            w.destroy()
        if not self._ctx:
            return
        msgs = self._get_messages()
        if not msgs:
            lbl = ctk.CTkLabel(self._msg_frame, text="No messages yet.\nBe the first to say something.",
                                font=FONT, text_color=TEXT_MUTED)
            lbl.grid(row=0, column=0, pady=40)
            return
        for i, m in enumerate(msgs):
            item = MessageItem(self._msg_frame, m)
            item.grid(row=i, column=0, sticky="ew", padx=12, pady=(4,0))
        # scroll to bottom
        self._msg_frame.after(50, lambda: self._msg_frame._parent_canvas.yview_moveto(1.0))

    def _get_messages(self) -> list:
        if not self._ctx or not self._state:
            return []
        t    = self._ctx.get("type","")
        cid  = self._ctx.get("id","")
        sid  = self._ctx.get("server_id","")
        convs = self._state.get("conversations", {})

        if t == "dm":
            return convs.get(cid, [])
        if t == "group":
            for g in self._state.get("groups", []):
                if g["id"] == cid:
                    return g.get("messages", [])
        if t == "channel":
            for s in self._state.get("servers", []):
                if s["id"] == sid:
                    for ch in s.get("channels", []):
                        if ch["id"] == cid:
                            return ch.get("messages", [])
        return []

    def _send(self):
        if not self._ctx or self._ctx.get("restricted"):
            return
        body = self._input.get().strip()
        if not body:
            return
        t   = self._ctx.get("type","")
        cid = self._ctx.get("id","")
        sid = self._ctx.get("server_id","")
        if t == "dm":
            self._ipc.send({"cmd":"send_dm","friend_id":cid,"body":body})
        elif t == "group":
            self._ipc.send({"cmd":"send_group","group_id":cid,"body":body})
        elif t == "channel":
            self._ipc.send({"cmd":"send_channel","server_id":sid,"channel_id":cid,"body":body})
        self._input.delete(0, tk.END)

    def update_state(self, state: dict):
        self._state = state
        if self._ctx:
            self._render_messages()

# ─── Sidebar ──────────────────────────────────────────────────────────────────

class Sidebar(ctk.CTkFrame):
    def __init__(self, parent, ipc: IPC, on_select, on_action):
        super().__init__(parent, fg_color=BG_DARK, width=240, corner_radius=0)
        self._ipc       = ipc
        self._on_select = on_select
        self._on_action = on_action
        self._state     = {}
        self._mode      = "home"    # 'home' or server_id
        self._selected  = None
        self.grid_propagate(False)
        self.grid_rowconfigure(1, weight=1)
        self.grid_columnconfigure(0, weight=1)

        # Section header
        self._header_frame = ctk.CTkFrame(self, fg_color=BG_DARKEST, height=48, corner_radius=0)
        self._header_frame.grid(row=0, column=0, sticky="ew")
        self._header_frame.grid_propagate(False)
        self._header_lbl = ctk.CTkLabel(self._header_frame, text="Direct Messages",
                                         font=("Segoe UI",13,"bold"), text_color=TEXT_BRIGHT)
        self._header_lbl.place(relx=0.5, rely=0.5, anchor="center")

        # Scrollable list
        self._list = ScrollFrame(self, width=236)
        self._list.grid(row=1, column=0, sticky="nsew")

        # Me-area
        self._me_frame = ctk.CTkFrame(self, fg_color=BG_DARKEST, height=52, corner_radius=0)
        self._me_frame.grid(row=2, column=0, sticky="ew")
        self._me_frame.grid_propagate(False)
        self._me_frame.grid_columnconfigure(1, weight=1)
        self._me_av   = ctk.CTkLabel(self._me_frame, text="?", width=32, height=32,
                                      fg_color=BLURPLE, corner_radius=16, text_color=TEXT_BRIGHT,
                                      font=("Segoe UI",12,"bold"))
        self._me_av.grid(row=0, column=0, padx=(10,6), pady=10)
        self._me_name = ctk.CTkLabel(self._me_frame, text="You", font=FONT_SM,
                                      text_color=TEXT_BRIGHT, anchor="w")
        self._me_name.grid(row=0, column=1, sticky="w")
        self._me_status = ctk.CTkLabel(self._me_frame, text="● online", font=FONT_XS,
                                        text_color=GREEN, anchor="w")
        self._me_status.grid(row=1, column=1, sticky="w", pady=(0,4))
        edit_btn = ctk.CTkButton(self._me_frame, text="✎", width=28, height=28,
                                  fg_color="transparent", text_color=TEXT_MUTED,
                                  hover_color=BG_HOVER,
                                  command=lambda: on_action("edit_profile", {}))
        edit_btn.grid(row=0, column=2, padx=6)

    def set_mode(self, mode: str, state: dict):
        self._mode  = mode
        self._state = state
        self._render()

    def _render(self):
        for w in self._list.winfo_children():
            w.destroy()

        if self._mode == "home":
            self._header_lbl.configure(text="Direct Messages")
            self._render_home()
        else:
            sv = next((s for s in self._state.get("servers",[]) if s["id"]==self._mode), None)
            if sv:
                self._header_lbl.configure(text=sv["name"])
                self._render_server(sv)

        # Me area
        s = self._state
        name = s.get("my_nick") or s.get("my_name","You")
        st   = s.get("my_status","online")
        initials = "".join(w[0].upper() for w in name.split()[:2]) if name else "?"
        self._me_av.configure(text=initials[:2])
        self._me_name.configure(text=name)
        self._me_status.configure(text=f"● {st}", text_color=status_color(st))

    def _render_home(self):
        row = 0
        # Friends section
        friends = self._state.get("friends", [])
        if friends:
            lbl = ctk.CTkLabel(self._list, text="FRIENDS", font=("Segoe UI",10,"bold"),
                                text_color=TEXT_MUTED)
            lbl.grid(row=row, column=0, sticky="w", padx=12, pady=(12,4))
            row += 1
            for f in friends:
                btn = self._channel_btn(self._list, f"@ {f['display_name']}", row,
                                         key=f["id"], ctx_type="dm",
                                         extra={"id":f["id"],"name":f["display_name"],"type":"dm"})
                row += 1

        # Groups section
        groups = self._state.get("groups", [])
        if groups:
            lbl = ctk.CTkLabel(self._list, text="GROUPS", font=("Segoe UI",10,"bold"),
                                text_color=TEXT_MUTED)
            lbl.grid(row=row, column=0, sticky="w", padx=12, pady=(12,4))
            row += 1
            for g in groups:
                btn = self._channel_btn(self._list, f"⊕ {g['name']}", row,
                                         key=g["id"], ctx_type="group",
                                         extra={"id":g["id"],"name":g["name"],"type":"group"})
                row += 1

        # Action buttons
        row_frame = ctk.CTkFrame(self._list, fg_color="transparent")
        row_frame.grid(row=row, column=0, sticky="ew", padx=8, pady=(16,4))
        ctk.CTkButton(row_frame, text="+ Friend", height=28, fg_color=BG_CARD,
                       hover_color=BG_HOVER, text_color=TEXT_MUTED, font=FONT_XS,
                       command=lambda: self._on_action("add_friend", {})).pack(side="left",padx=2)
        ctk.CTkButton(row_frame, text="+ Group", height=28, fg_color=BG_CARD,
                       hover_color=BG_HOVER, text_color=TEXT_MUTED, font=FONT_XS,
                       command=lambda: self._on_action("create_group", {})).pack(side="left",padx=2)
        ctk.CTkButton(row_frame, text="Join Server", height=28, fg_color=BG_CARD,
                       hover_color=BG_HOVER, text_color=TEXT_MUTED, font=FONT_XS,
                       command=lambda: self._on_action("join_server", {})).pack(side="left",padx=2)

    def _render_server(self, sv: dict):
        my_id    = self._state.get("my_id","")
        is_owner = sv.get("owner_id","") == my_id
        # Channel list header
        row = 0
        ctk.CTkLabel(self._list, text="CHANNELS", font=("Segoe UI",10,"bold"),
                      text_color=TEXT_MUTED).grid(row=row,column=0,sticky="w",padx=12,pady=(12,4))
        row += 1
        ann_ch_id = None
        for ch in sv.get("channels",[]):
            ch_type   = ch.get("ch_type","public")
            restricted = (ch_type == "announcement") and not is_owner
            icon = "📣" if ch_type=="announcement" else "#"
            name_str = f"{icon} {ch['name']}"
            if restricted:
                name_str = f"🔒 {ch['name']}"
            ctx = {"type":"channel","id":ch["id"],"name":ch["name"],
                   "topic":ch.get("topic",""),"server_id":sv["id"],
                   "ch_type":ch_type,"restricted":restricted}
            self._channel_btn(self._list, name_str, row, key=ch["id"],
                               ctx_type="channel", extra=ctx)
            row += 1

        # Server code
        ctk.CTkLabel(self._list, text=f"Code: {sv.get('server_code','')}", font=MONO,
                      text_color=TEXT_MUTED).grid(row=row,column=0,sticky="w",padx=12,pady=(10,2))
        row += 1
        # Members
        ctk.CTkLabel(self._list, text="MEMBERS", font=("Segoe UI",10,"bold"),
                      text_color=TEXT_MUTED).grid(row=row,column=0,sticky="w",padx=12,pady=(12,4))
        row += 1
        for m in sv.get("members",[])[:15]:
            mc = ctk.CTkFrame(self._list, fg_color="transparent", height=32)
            mc.grid(row=row, column=0, sticky="ew", padx=8, pady=1)
            mc.grid_columnconfigure(1, weight=1)
            ctk.CTkLabel(mc,text="●",text_color=GREEN if not m.get("muted") else TEXT_MUTED,
                          font=FONT_XS,width=14).grid(row=0,column=0,padx=(6,4))
            ctk.CTkLabel(mc,text=m.get("display_name","?"),font=FONT_SM,text_color=TEXT,
                          anchor="w").grid(row=0,column=1,sticky="w")
            role = m.get("role","")
            if role in ("owner","admin","moderator"):
                role_colors = {"owner":YELLOW,"admin":RED,"moderator":BLURPLE}
                ctk.CTkLabel(mc,text=role[:3].upper(),font=("Segoe UI",9,"bold"),
                              text_color=role_colors.get(role,TEXT_MUTED)).grid(row=0,column=2,padx=4)
            row += 1
        if is_owner:
            ctk.CTkButton(self._list, text="Manage Members", height=28, fg_color=BG_CARD,
                           hover_color=BG_HOVER, text_color=TEXT_MUTED, font=FONT_XS,
                           command=lambda: self._on_action("manage_members",
                                                            {"server_id":sv["id"],"members":sv.get("members",[])})).grid(
                row=row, column=0, sticky="ew", padx=8, pady=(8,4))

    def _channel_btn(self, parent, label: str, row: int, key: str, ctx_type: str, extra: dict):
        is_sel = (self._selected == key)
        bg = BG_SELECTED if is_sel else "transparent"
        btn = ctk.CTkButton(parent, text=label, height=32, fg_color=bg,
                             hover_color=BG_HOVER, text_color=TEXT if is_sel else TEXT_MUTED,
                             anchor="w", font=FONT_SM, corner_radius=4)
        btn.grid(row=row, column=0, sticky="ew", padx=8, pady=1)
        btn.configure(command=lambda k=key, e=extra: self._select(k, e))
        # Unread badge
        unread = self._state.get("unread",{}).get(key,0)
        if unread:
            badge = ctk.CTkLabel(parent, text=str(unread), width=18, height=18,
                                  fg_color=RED, corner_radius=9, font=("Segoe UI",9,"bold"),
                                  text_color=TEXT_BRIGHT)
            badge.grid(row=row, column=0, sticky="e", padx=12)
        return btn

    def _select(self, key: str, ctx: dict):
        self._selected = key
        self._on_select(ctx)
        self._render()

    def update_state(self, state: dict):
        self._state = state
        self._render()


# ─── Left rail (server icons) ─────────────────────────────────────────────────

class Rail(ctk.CTkFrame):
    def __init__(self, parent, on_select):
        super().__init__(parent, fg_color=BG_DARKEST, width=68, corner_radius=0)
        self._on_select  = on_select
        self._state      = {}
        self._selected   = "home"
        self.grid_propagate(False)

        # Home button
        self._home_btn = self._icon_btn("⌂", "home", tooltip="Home")
        self._home_btn.pack(pady=(12,4))
        self._divider()

        # Server icons area (will be populated)
        self._servers_frame = ctk.CTkFrame(self, fg_color="transparent")
        self._servers_frame.pack(fill="x")

        self._divider()
        # Join server
        join_btn = ctk.CTkButton(self, text="+", width=46, height=46, fg_color=BG_DARK,
                                  hover_color=GREEN, text_color=GREEN, corner_radius=23,
                                  font=("Segoe UI",20,"bold"),
                                  command=lambda: on_select("join_server"))
        join_btn.pack(pady=4)

        # Tor indicator (bottom)
        self._tor_lbl = ctk.CTkLabel(self, text="●", font=("Segoe UI",18),
                                      text_color=TEXT_MUTED)
        self._tor_lbl.pack(side="bottom", pady=14)
        self._tor_tip = ctk.CTkLabel(self, text="Tor: offline", font=FONT_XS,
                                      text_color=TEXT_MUTED)
        self._tor_tip.pack(side="bottom", pady=(0,0))

    def _divider(self):
        f = ctk.CTkFrame(self, fg_color=DIVIDER, height=2, width=32)
        f.pack(pady=4)

    def _icon_btn(self, text: str, sid: str, tooltip="") -> ctk.CTkButton:
        is_sel = (self._selected == sid)
        btn = ctk.CTkButton(self, text=text, width=46, height=46,
                             fg_color=BLURPLE if is_sel else BG_DARK,
                             hover_color=BLURPLE, text_color=TEXT_BRIGHT,
                             corner_radius=23 if not is_sel else 12,
                             font=("Segoe UI",18,"bold"))
        btn.configure(command=lambda s=sid: self._select(s))
        return btn

    def _select(self, sid: str):
        self._selected = sid
        self._on_select(sid)
        self._refresh_icons()

    def _refresh_icons(self):
        for w in self._servers_frame.winfo_children():
            w.destroy()
        for sv in self._state.get("servers",[]):
            initials = "".join(x[0].upper() for x in sv["name"].split()[:2])
            is_sel = (self._selected == sv["id"])
            btn = ctk.CTkButton(self._servers_frame, text=initials, width=46, height=46,
                                 fg_color=BLURPLE if is_sel else BG_DARK,
                                 hover_color=BLURPLE, text_color=TEXT_BRIGHT,
                                 corner_radius=12 if is_sel else 23,
                                 font=("Segoe UI",13,"bold"))
            btn.configure(command=lambda s=sv["id"]: self._select(s))
            btn.pack(pady=4)

    def update_state(self, state: dict):
        self._state = state
        self._refresh_icons()
        tor = state.get("tor_status","offline")
        color = {"connected":GREEN,"connecting":YELLOW,"starting":YELLOW,"error":RED}.get(tor, TEXT_MUTED)
        self._tor_lbl.configure(text_color=color)
        self._tor_tip.configure(text=f"Tor: {tor}")

# ─── Main window ──────────────────────────────────────────────────────────────

class MainWindow(ctk.CTkFrame):
    def __init__(self, parent, ipc: IPC):
        super().__init__(parent, fg_color=BG_DARKEST, corner_radius=0)
        self._ipc   = ipc
        self._state = {}
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(2, weight=1)

        self._rail    = Rail(self, self._rail_select)
        self._rail.grid(row=0, column=0, sticky="ns")

        self._sidebar = Sidebar(self, ipc, self._open_ctx, self._sidebar_action)
        self._sidebar.grid(row=0, column=1, sticky="ns")

        self._chat    = ChatArea(self, ipc)
        self._chat.grid(row=0, column=2, sticky="nsew")

    def update_state(self, state: dict):
        self._state = state
        self._rail.update_state(state)
        mode = self._sidebar._mode
        self._sidebar.set_mode(mode, state)
        self._chat.update_state(state)

    def _rail_select(self, sid: str):
        if sid in ("home", "join_server"):
            if sid == "join_server":
                self._sidebar_action("join_server", {})
            else:
                self._sidebar.set_mode("home", self._state)
        else:
            self._sidebar.set_mode(sid, self._state)

    def _open_ctx(self, ctx: dict):
        self._chat.set_context(ctx, self._state)
        # clear unread
        cid = ctx.get("id","")
        if cid in self._state.get("unread",{}):
            self._state["unread"].pop(cid, None)

    def _sidebar_action(self, action: str, data: dict):
        root = self.winfo_toplevel()
        if action == "add_friend":
            AddFriendModal(root, lambda uid, name: self._ipc.send(
                {"cmd":"add_friend","user_id":uid,"name":name}))
        elif action == "create_group":
            CreateGroupModal(root, lambda name, desc: self._ipc.send(
                {"cmd":"create_group","name":name,"desc":desc}))
        elif action == "join_server":
            JoinServerModal(root, lambda code: self._ipc.send(
                {"cmd":"join_server","code":code}))
        elif action == "edit_profile":
            EditProfileModal(root, self._state,
                              lambda n,nk,b,s: (self._ipc.send({"cmd":"save_profile","name":n,"nick":nk,"bio":b}),
                                                self._ipc.send({"cmd":"set_status","status":s})))
        elif action == "manage_members":
            my_id    = self._state.get("my_id","")
            members  = data.get("members",[])
            sid      = data.get("server_id","")
            sv       = next((s for s in self._state.get("servers",[]) if s["id"]==sid), {})
            is_owner = sv.get("owner_id","") == my_id
            MembersModal(root, sv.get("name","Server"), members, my_id, is_owner,
                          lambda uid, nm: self._show_member_menu(uid, nm, sid, is_owner))

    def _show_member_menu(self, user_id: str, name: str, server_id: str, is_owner: bool):
        if not is_owner:
            return
        root = self.winfo_toplevel()
        menu = ContextMenu(root)
        menu.add_command(label=f"Kick {name}",
                          command=lambda: self._ipc.send({"cmd":"kick","context_id":server_id,"user_id":user_id,"is_server":True}))
        menu.add_command(label=f"Ban {name}",
                          command=lambda: self._ipc.send({"cmd":"ban","context_id":server_id,"user_id":user_id,"is_server":True}))
        menu.add_separator()
        menu.add_command(label="Set Admin",
                          command=lambda: self._ipc.send({"cmd":"set_role","context_id":server_id,"user_id":user_id,"role":"admin","is_server":True}))
        menu.add_command(label="Set Moderator",
                          command=lambda: self._ipc.send({"cmd":"set_role","context_id":server_id,"user_id":user_id,"role":"moderator","is_server":True}))
        menu.add_command(label="Remove Role",
                          command=lambda: self._ipc.send({"cmd":"set_role","context_id":server_id,"user_id":user_id,"role":"member","is_server":True}))
        # show at cursor
        try:
            import pyautogui
            x, y = pyautogui.position()
        except Exception:
            x, y = root.winfo_pointerx(), root.winfo_pointery()
        menu.post(x, y)

# ─── App root ─────────────────────────────────────────────────────────────────

class NullChatApp(ctk.CTk):
    def __init__(self, port: int):
        super().__init__()
        self.title("NullChat")
        self.geometry("1100x720")
        self.minsize(800, 560)
        self.configure(fg_color=BG_DARKEST)
        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")

        self._ipc     = IPC(port)
        self._phase   = "loading"
        self._state   = {}
        self._screen  = None

        self._show_loading()
        self.after(100, self._connect)

    # ── screen management ─────────────────────────────────────────────────────

    def _clear(self):
        if self._screen:
            self._screen.destroy()
            self._screen = None

    def _show_loading(self):
        self._clear()
        self._screen = LoadingScreen(self)
        self._screen.pack(fill="both", expand=True)

    def _show_setup(self, is_first: bool):
        self._clear()
        self._screen = SetupScreen(self, self._ipc, is_first, lambda: None)
        self._screen.pack(fill="both", expand=True)

    def _show_main(self):
        self._clear()
        self._screen = MainWindow(self, self._ipc)
        self._screen.pack(fill="both", expand=True)
        if self._state:
            self._screen.update_state(self._state)

    # ── connection ────────────────────────────────────────────────────────────

    def _connect(self):
        def _do():
            ok = self._ipc.connect(timeout=30)
            self.after(0, self._on_connected if ok else self._on_connect_failed)
        threading.Thread(target=_do, daemon=True).start()

    def _on_connected(self):
        self._screen.set_status("Connected — waiting for backend…")
        self._ipc.send({"cmd":"get_state"})
        threading.Thread(target=self._event_loop, daemon=True).start()

    def _on_connect_failed(self):
        self._clear()
        f = ctk.CTkFrame(self, fg_color=BG_DARKEST)
        f.pack(fill="both", expand=True)
        ctk.CTkLabel(f, text="⬡", font=("Segoe UI",60), text_color=RED).pack(pady=(120,0))
        ctk.CTkLabel(f, text="Could not connect to NullChat backend.",
                     font=FONT_LG, text_color=TEXT_BRIGHT).pack()
        ctk.CTkLabel(f, text="Make sure the Rust backend is running.",
                     font=FONT_SM, text_color=TEXT_MUTED).pack(pady=4)
        ctk.CTkButton(f, text="Retry", fg_color=BLURPLE, command=lambda:(self._show_loading(), self.after(100,self._connect))).pack(pady=12)

    # ── event loop ────────────────────────────────────────────────────────────

    def _event_loop(self):
        while self._ipc._alive:
            ev = self._ipc.readline()
            if ev is None:
                break
            self.after(0, lambda e=ev: self._handle_event(e))
        if self._ipc._alive:
            self.after(0, self._on_connect_failed)

    def _handle_event(self, ev: dict):
        kind = ev.get("event","")

        if kind == "phase":
            phase = ev.get("phase","")
            if phase == "setup":
                self._show_setup(is_first=True)
            elif phase == "unlock":
                self._show_setup(is_first=False)
            elif phase == "loading":
                if isinstance(self._screen, LoadingScreen):
                    self._screen.set_status("Unlocking vault…")
            elif phase == "tor":
                if isinstance(self._screen, LoadingScreen):
                    self._screen.set_status("Starting Tor…")
            elif phase == "main":
                self._show_main()

        elif kind == "state":
            self._state = ev.get("data", {})
            if isinstance(self._screen, LoadingScreen):
                if self._phase != "main":
                    # If we get state but are still loading, maybe we need setup
                    pass
            elif isinstance(self._screen, MainWindow):
                self._screen.update_state(self._state)
            else:
                # Received state unexpectedly — check if we should transition
                pass

        elif kind == "tor":
            tor_status = ev.get("status","")
            onion      = ev.get("onion")
            if isinstance(self._screen, LoadingScreen):
                msg = f"Tor: {tor_status}"
                if onion:
                    msg += f" ({onion[:12]}…)"
                self._screen.set_status(msg)
            if self._state:
                self._state["tor_status"] = tor_status

        elif kind == "notif":
            msg = ev.get("msg","")
            # Show subtle toast
            self._toast(msg, color=BLURPLE)

        elif kind == "error":
            msg = ev.get("msg","")
            if isinstance(self._screen, SetupScreen):
                self._screen.show_error(msg)
            else:
                self._toast(msg, color=RED)

    def _toast(self, msg: str, color=BLURPLE):
        toast = ctk.CTkFrame(self, fg_color=color, corner_radius=8, width=320, height=44)
        toast.place(relx=0.5, rely=0.97, anchor="s")
        ctk.CTkLabel(toast, text=msg, font=FONT_SM, text_color=TEXT_BRIGHT,
                     wraplength=300).place(relx=0.5, rely=0.5, anchor="center")
        self.after(3000, toast.destroy)

    def on_close(self):
        self._ipc.close()
        self.destroy()

# ─── Entry point ─────────────────────────────────────────────────────────────

def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 17778
    app  = NullChatApp(port)
    app.protocol("WM_DELETE_WINDOW", app.on_close)
    app.mainloop()

if __name__ == "__main__":
    main()
