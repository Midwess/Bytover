//
//  ShareView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

public struct ShareView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject var core: Core
    @State private var startTime = Date.now
    
    public var body: some View {
        ZStack(alignment: .bottom) {
            Theme.darkBgGradient.opacity(0.3)
            StunningBackgroundGradientAnimation()
            ScrollView(.vertical) {
                VStack(alignment: .center) {
                    Rectangle()
                        .fill(.clear)
                        .frame(width: 10, height: safeAreaInsets.top + 10)
                    LogoView(width: 60)
                        .frame(width: .infinity, height: 100)
                    
                    Text("Your best file transfer")
                        .multilineTextAlignment(.center)
                        .modifier(Body1())
                        .foregroundStyle(Theme.gradientHeading)
                        .padding(.top, 5)
                    
                    UpgradePremiumButton()
                        .padding(.top, 5)
                    
                    Spacer().frame(height: 20)
                    
                    ContentPickerView()
                        .padding(.horizontal, 19)

                    ForEach(core.transfer?.selected_resources ?? [], id: \.self) { item in
                        SelectedResourceItem(resource: item)
                            .padding(.horizontal, 24)
                            .padding(.top, 16)
                    }
                }
            }
            
            ShareButton(width: 150)
                .padding(.bottom, 80)
        }
        .onAppear() {
            print("On appear")
            core.update(.transfer(.initSession))
        }
        .ignoresSafeArea()
    }
}

#Preview {
   ShareView()
        .environmentObject(CoreMock.withSelectedFileTransfers())
}
