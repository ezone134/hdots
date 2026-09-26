/****************************************************************************
** Meta object code from reading C++ file 'qsnet.hpp'
**
** Created by: The Qt Meta Object Compiler version 69 (Qt 6.11.1)
**
** WARNING! All changes made in this file will be lost!
*****************************************************************************/

#include "../../../qsnet.hpp"
#include <QtNetwork/QSslError>
#include <QtCore/qmetatype.h>

#include <QtCore/qtmochelpers.h>

#include <memory>


#include <QtCore/qxptype_traits.h>
#if !defined(Q_MOC_OUTPUT_REVISION)
#error "The header file 'qsnet.hpp' doesn't include <QObject>."
#elif Q_MOC_OUTPUT_REVISION != 69
#error "This file was generated using the moc from 6.11.1. It"
#error "cannot be used with the include files from this version of Qt."
#error "(The moc has changed too much.)"
#endif

#ifndef Q_CONSTINIT
#define Q_CONSTINIT
#endif

QT_WARNING_PUSH
QT_WARNING_DISABLE_DEPRECATED
QT_WARNING_DISABLE_GCC("-Wuseless-cast")
namespace {
struct qt_meta_tag_ZN5QsNetE_t {};
} // unnamed namespace

template <> constexpr inline auto QsNet::qt_create_metaobjectdata<qt_meta_tag_ZN5QsNetE_t>()
{
    namespace QMC = QtMocConstants;
    QtMocHelpers::StringRefStorage qt_stringData {
        "QsNet",
        "wifiEnabledChanged",
        "",
        "wifiScanningChanged",
        "bluetoothEnabledChanged",
        "onNmPropsChanged",
        "QDBusMessage",
        "message",
        "onBtPropsChanged",
        "httpGet",
        "url",
        "timeoutMs",
        "QJSValue",
        "callback",
        "refreshWifiEnabled",
        "toggleWifi",
        "scanWifi",
        "wifiSignal",
        "connectWifi",
        "ssid",
        "password",
        "disconnectWifi",
        "refreshBluetoothEnabled",
        "toggleBluetooth",
        "scanBluetooth",
        "btConnectedName",
        "connectBluetooth",
        "address",
        "disconnectBluetooth",
        "wifiEnabled",
        "wifiScanning",
        "bluetoothEnabled"
    };

    QtMocHelpers::UintData qt_methods {
        // Signal 'wifiEnabledChanged'
        QtMocHelpers::SignalData<void()>(1, 2, QMC::AccessPublic, QMetaType::Void),
        // Signal 'wifiScanningChanged'
        QtMocHelpers::SignalData<void()>(3, 2, QMC::AccessPublic, QMetaType::Void),
        // Signal 'bluetoothEnabledChanged'
        QtMocHelpers::SignalData<void()>(4, 2, QMC::AccessPublic, QMetaType::Void),
        // Slot 'onNmPropsChanged'
        QtMocHelpers::SlotData<void(const QDBusMessage &)>(5, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 6, 7 },
        }}),
        // Slot 'onBtPropsChanged'
        QtMocHelpers::SlotData<void(const QDBusMessage &)>(8, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 6, 7 },
        }}),
        // Method 'httpGet'
        QtMocHelpers::MethodData<void(const QString &, int, const QJSValue &)>(9, 2, QMC::AccessPublic, QMetaType::Void, {{
            { QMetaType::QString, 10 }, { QMetaType::Int, 11 }, { 0x80000000 | 12, 13 },
        }}),
        // Method 'httpGet'
        QtMocHelpers::MethodData<void(const QString &, int)>(9, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void, {{
            { QMetaType::QString, 10 }, { QMetaType::Int, 11 },
        }}),
        // Method 'httpGet'
        QtMocHelpers::MethodData<void(const QString &)>(9, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void, {{
            { QMetaType::QString, 10 },
        }}),
        // Method 'refreshWifiEnabled'
        QtMocHelpers::MethodData<void()>(14, 2, QMC::AccessPublic, QMetaType::Void),
        // Method 'toggleWifi'
        QtMocHelpers::MethodData<void()>(15, 2, QMC::AccessPublic, QMetaType::Void),
        // Method 'scanWifi'
        QtMocHelpers::MethodData<void(const QJSValue &)>(16, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 12, 13 },
        }}),
        // Method 'scanWifi'
        QtMocHelpers::MethodData<void()>(16, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void),
        // Method 'wifiSignal'
        QtMocHelpers::MethodData<void(const QJSValue &)>(17, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 12, 13 },
        }}),
        // Method 'wifiSignal'
        QtMocHelpers::MethodData<void()>(17, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void),
        // Method 'connectWifi'
        QtMocHelpers::MethodData<void(const QString &, const QString &, const QJSValue &)>(18, 2, QMC::AccessPublic, QMetaType::Void, {{
            { QMetaType::QString, 19 }, { QMetaType::QString, 20 }, { 0x80000000 | 12, 13 },
        }}),
        // Method 'connectWifi'
        QtMocHelpers::MethodData<void(const QString &, const QString &)>(18, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void, {{
            { QMetaType::QString, 19 }, { QMetaType::QString, 20 },
        }}),
        // Method 'connectWifi'
        QtMocHelpers::MethodData<void(const QString &)>(18, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void, {{
            { QMetaType::QString, 19 },
        }}),
        // Method 'disconnectWifi'
        QtMocHelpers::MethodData<void(const QJSValue &)>(21, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 12, 13 },
        }}),
        // Method 'disconnectWifi'
        QtMocHelpers::MethodData<void()>(21, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void),
        // Method 'refreshBluetoothEnabled'
        QtMocHelpers::MethodData<void()>(22, 2, QMC::AccessPublic, QMetaType::Void),
        // Method 'toggleBluetooth'
        QtMocHelpers::MethodData<void()>(23, 2, QMC::AccessPublic, QMetaType::Void),
        // Method 'scanBluetooth'
        QtMocHelpers::MethodData<void(const QJSValue &)>(24, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 12, 13 },
        }}),
        // Method 'scanBluetooth'
        QtMocHelpers::MethodData<void()>(24, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void),
        // Method 'btConnectedName'
        QtMocHelpers::MethodData<void(const QJSValue &)>(25, 2, QMC::AccessPublic, QMetaType::Void, {{
            { 0x80000000 | 12, 13 },
        }}),
        // Method 'btConnectedName'
        QtMocHelpers::MethodData<void()>(25, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void),
        // Method 'connectBluetooth'
        QtMocHelpers::MethodData<void(const QString &, const QJSValue &)>(26, 2, QMC::AccessPublic, QMetaType::Void, {{
            { QMetaType::QString, 27 }, { 0x80000000 | 12, 13 },
        }}),
        // Method 'connectBluetooth'
        QtMocHelpers::MethodData<void(const QString &)>(26, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void, {{
            { QMetaType::QString, 27 },
        }}),
        // Method 'disconnectBluetooth'
        QtMocHelpers::MethodData<void(const QString &, const QJSValue &)>(28, 2, QMC::AccessPublic, QMetaType::Void, {{
            { QMetaType::QString, 27 }, { 0x80000000 | 12, 13 },
        }}),
        // Method 'disconnectBluetooth'
        QtMocHelpers::MethodData<void(const QString &)>(28, 2, QMC::AccessPublic | QMC::MethodCloned, QMetaType::Void, {{
            { QMetaType::QString, 27 },
        }}),
    };
    QtMocHelpers::UintData qt_properties {
        // property 'wifiEnabled'
        QtMocHelpers::PropertyData<bool>(29, QMetaType::Bool, QMC::DefaultPropertyFlags, 0),
        // property 'wifiScanning'
        QtMocHelpers::PropertyData<bool>(30, QMetaType::Bool, QMC::DefaultPropertyFlags, 1),
        // property 'bluetoothEnabled'
        QtMocHelpers::PropertyData<bool>(31, QMetaType::Bool, QMC::DefaultPropertyFlags, 2),
    };
    QtMocHelpers::UintData qt_enums {
    };
    return QtMocHelpers::metaObjectData<QsNet, qt_meta_tag_ZN5QsNetE_t>(QMC::MetaObjectFlag{}, qt_stringData,
            qt_methods, qt_properties, qt_enums);
}
Q_CONSTINIT const QMetaObject QsNet::staticMetaObject = { {
    QMetaObject::SuperData::link<QObject::staticMetaObject>(),
    qt_staticMetaObjectStaticContent<qt_meta_tag_ZN5QsNetE_t>.stringdata,
    qt_staticMetaObjectStaticContent<qt_meta_tag_ZN5QsNetE_t>.data,
    qt_static_metacall,
    nullptr,
    qt_staticMetaObjectRelocatingContent<qt_meta_tag_ZN5QsNetE_t>.metaTypes,
    nullptr
} };

void QsNet::qt_static_metacall(QObject *_o, QMetaObject::Call _c, int _id, void **_a)
{
    auto *_t = static_cast<QsNet *>(_o);
    if (_c == QMetaObject::InvokeMetaMethod) {
        switch (_id) {
        case 0: _t->wifiEnabledChanged(); break;
        case 1: _t->wifiScanningChanged(); break;
        case 2: _t->bluetoothEnabledChanged(); break;
        case 3: _t->onNmPropsChanged((*reinterpret_cast<std::add_pointer_t<QDBusMessage>>(_a[1]))); break;
        case 4: _t->onBtPropsChanged((*reinterpret_cast<std::add_pointer_t<QDBusMessage>>(_a[1]))); break;
        case 5: _t->httpGet((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1])),(*reinterpret_cast<std::add_pointer_t<int>>(_a[2])),(*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[3]))); break;
        case 6: _t->httpGet((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1])),(*reinterpret_cast<std::add_pointer_t<int>>(_a[2]))); break;
        case 7: _t->httpGet((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1]))); break;
        case 8: _t->refreshWifiEnabled(); break;
        case 9: _t->toggleWifi(); break;
        case 10: _t->scanWifi((*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[1]))); break;
        case 11: _t->scanWifi(); break;
        case 12: _t->wifiSignal((*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[1]))); break;
        case 13: _t->wifiSignal(); break;
        case 14: _t->connectWifi((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1])),(*reinterpret_cast<std::add_pointer_t<QString>>(_a[2])),(*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[3]))); break;
        case 15: _t->connectWifi((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1])),(*reinterpret_cast<std::add_pointer_t<QString>>(_a[2]))); break;
        case 16: _t->connectWifi((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1]))); break;
        case 17: _t->disconnectWifi((*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[1]))); break;
        case 18: _t->disconnectWifi(); break;
        case 19: _t->refreshBluetoothEnabled(); break;
        case 20: _t->toggleBluetooth(); break;
        case 21: _t->scanBluetooth((*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[1]))); break;
        case 22: _t->scanBluetooth(); break;
        case 23: _t->btConnectedName((*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[1]))); break;
        case 24: _t->btConnectedName(); break;
        case 25: _t->connectBluetooth((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1])),(*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[2]))); break;
        case 26: _t->connectBluetooth((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1]))); break;
        case 27: _t->disconnectBluetooth((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1])),(*reinterpret_cast<std::add_pointer_t<QJSValue>>(_a[2]))); break;
        case 28: _t->disconnectBluetooth((*reinterpret_cast<std::add_pointer_t<QString>>(_a[1]))); break;
        default: ;
        }
    }
    if (_c == QMetaObject::RegisterMethodArgumentMetaType) {
        switch (_id) {
        default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
        case 3:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QDBusMessage >(); break;
            }
            break;
        case 4:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QDBusMessage >(); break;
            }
            break;
        case 5:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 2:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 10:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 12:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 14:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 2:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 17:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 21:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 23:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 0:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 25:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 1:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        case 27:
            switch (*reinterpret_cast<int*>(_a[1])) {
            default: *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType(); break;
            case 1:
                *reinterpret_cast<QMetaType *>(_a[0]) = QMetaType::fromType< QJSValue >(); break;
            }
            break;
        }
    }
    if (_c == QMetaObject::IndexOfMethod) {
        if (QtMocHelpers::indexOfMethod<void (QsNet::*)()>(_a, &QsNet::wifiEnabledChanged, 0))
            return;
        if (QtMocHelpers::indexOfMethod<void (QsNet::*)()>(_a, &QsNet::wifiScanningChanged, 1))
            return;
        if (QtMocHelpers::indexOfMethod<void (QsNet::*)()>(_a, &QsNet::bluetoothEnabledChanged, 2))
            return;
    }
    if (_c == QMetaObject::ReadProperty) {
        void *_v = _a[0];
        switch (_id) {
        case 0: *reinterpret_cast<bool*>(_v) = _t->wifiEnabled(); break;
        case 1: *reinterpret_cast<bool*>(_v) = _t->wifiScanning(); break;
        case 2: *reinterpret_cast<bool*>(_v) = _t->bluetoothEnabled(); break;
        default: break;
        }
    }
}

const QMetaObject *QsNet::metaObject() const
{
    return QObject::d_ptr->metaObject ? QObject::d_ptr->dynamicMetaObject() : &staticMetaObject;
}

void *QsNet::qt_metacast(const char *_clname)
{
    if (!_clname) return nullptr;
    if (!strcmp(_clname, qt_staticMetaObjectStaticContent<qt_meta_tag_ZN5QsNetE_t>.strings))
        return static_cast<void*>(this);
    return QObject::qt_metacast(_clname);
}

int QsNet::qt_metacall(QMetaObject::Call _c, int _id, void **_a)
{
    _id = QObject::qt_metacall(_c, _id, _a);
    if (_id < 0)
        return _id;
    if (_c == QMetaObject::InvokeMetaMethod) {
        if (_id < 29)
            qt_static_metacall(this, _c, _id, _a);
        _id -= 29;
    }
    if (_c == QMetaObject::RegisterMethodArgumentMetaType) {
        if (_id < 29)
            qt_static_metacall(this, _c, _id, _a);
        _id -= 29;
    }
    if (_c == QMetaObject::ReadProperty || _c == QMetaObject::WriteProperty
            || _c == QMetaObject::ResetProperty || _c == QMetaObject::BindableProperty
            || _c == QMetaObject::RegisterPropertyMetaType) {
        qt_static_metacall(this, _c, _id, _a);
        _id -= 3;
    }
    return _id;
}

// SIGNAL 0
void QsNet::wifiEnabledChanged()
{
    QMetaObject::activate(this, &staticMetaObject, 0, nullptr);
}

// SIGNAL 1
void QsNet::wifiScanningChanged()
{
    QMetaObject::activate(this, &staticMetaObject, 1, nullptr);
}

// SIGNAL 2
void QsNet::bluetoothEnabledChanged()
{
    QMetaObject::activate(this, &staticMetaObject, 2, nullptr);
}
QT_WARNING_POP
