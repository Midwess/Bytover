//
//  TabBar.swift
//  BitBridge
//
//  Created by Tien Dang on 5/18/25.
//

import SwiftUI
import Foundation

struct CustomTabBar: View {
    @Binding var selection: Int

    @Environment(\.safeAreaInsets) private var safeAreaInsets

    var body: some View {
        HStack {
            ButtonNavigation(icon: ImageAsset.GlobeEmpty.image, icon_selected: ImageAsset.GlobeFull.image, index: 1, selection: $selection)
                .scaleEffect(1.08)
            Spacer()
            ButtonNavigation(icon: ImageAsset.MailReceiveEmpty.image, icon_selected: ImageAsset.MailReceiveFull.image, index: 2, selection: $selection)
            Spacer()
            ButtonNavigation(icon: ImageAsset.PuzzelEmpty.image, icon_selected: ImageAsset.PuzzelFull.image, index: 3, selection: $selection)
            Spacer()
            ButtonNavigation(icon: ImageAsset.SettingEmpty.image, icon_selected: ImageAsset.SettingFull.image, index: 4, selection: $selection)
        }
        .padding(.horizontal, SpaceTheme.screen.value)
        .padding(.bottom, safeAreaInsets.bottom)
    }
}

struct ButtonNavigation: View {
    var icon: Image
    var icon_selected: Image
    var index: Int

    @Binding var selection: Int

    var isSelected: Bool {
        index == selection
    }

    var body: some View {
        Button(action: {
            selection = index
        }) {
            (isSelected ? icon_selected : icon)
                .resizable()
                .scaledToFit()
                .foregroundStyle(.white)
                .frame(height: 24)
        }
        .frame(width: 44, height: 44)
    }
}
