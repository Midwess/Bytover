//
//  HomeView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 18/2/25.
//

import Foundation
import SwiftUI

struct HomeView: View {
    @EnvironmentObject private var core: Core
    @State private var selectedTab: Int = 1
    @State private var previousTab: Int = 1

    init() {
        UITabBar.appearance().unselectedItemTintColor = Theme.LightViolet.uiColor
            .withAlphaComponent(0.4)
    }

    var body: some View {
        ZStack {
            StunningBackgroundGradient()
            VStack(spacing: 0) {
                TabView(selection: $selectedTab) {
                    ShareView()
                        .tag(1)
                    ReceiveView()
                        .tag(2)
                    SettingView()
                        .tag(3)
                    SettingView()
                        .tag(4)
                }
                .tabViewStyle(PageTabViewStyle(indexDisplayMode: .never))
                .animation(.easeInOut, value: selectedTab)
                CustomTabBar(selection: Binding(
                    get: { selectedTab },
                    set: { newValue in
                        withAnimation(.easeInOut) {
                            // Determine slide direction based on tab order
                            let slideDirection = if newValue > previousTab {
                                Edge.trailing
                            } else if newValue < previousTab {
                                Edge.leading
                            } else {
                                Edge.leading
                            }

                            // Apply transition based on direction
                            withAnimation(.easeInOut) {
                                selectedTab = newValue
                            }
                            previousTab = newValue
                        }
                    }
                ))
            }
        }
        .ignoresSafeArea()
        .accentColor(Theme.LightViolet.color)
    }
}

#Preview("HomeView") {
    HomeView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)
}
